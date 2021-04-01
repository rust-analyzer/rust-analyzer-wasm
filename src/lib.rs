#![cfg(target_arch = "wasm32")]
#![allow(non_snake_case)]

use ide::{Analysis, CompletionConfig, DiagnosticsConfig, FileId, FilePosition, Indel, TextSize};
use ide_db::helpers::{
    insert_use::{InsertUseConfig, MergeBehavior, PrefixKind},
    SnippetCap,
};
use wasm_bindgen::prelude::*;

mod to_proto;

mod return_types;
use return_types::*;

pub use wasm_bindgen_rayon::init_thread_pool;

#[wasm_bindgen(start)]
pub fn start() {
    #[cfg(feature = "dev")]
    {
        console_error_panic_hook::set_once();
    }
    log::info!("worker initialized")
}

#[wasm_bindgen]
pub struct WorldState {
    analysis: Analysis,
    file_id: FileId,
}

#[wasm_bindgen]
impl WorldState {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        let (analysis, file_id) = Analysis::from_single_file("".to_owned());
        Self { analysis, file_id }
    }

    pub fn update(&mut self, code: String) -> JsValue {
        log::warn!("update");
        let (analysis, file_id) = Analysis::from_single_file(code);
        self.analysis = analysis;
        self.file_id = file_id;

        let line_index = self.analysis.file_line_index(self.file_id).unwrap();

        let highlights: Vec<_> = self
            .analysis
            .highlight(file_id)
            .unwrap()
            .into_iter()
            .map(|hl| Highlight {
                tag: Some(hl.highlight.tag.to_string()),
                range: to_proto::text_range(hl.range, &line_index),
            })
            .collect();

        let config = DiagnosticsConfig::default();

        let diagnostics: Vec<_> = self
            .analysis
            .diagnostics(&config, file_id)
            .unwrap()
            .into_iter()
            .map(|d| {
                let Range { startLineNumber, startColumn, endLineNumber, endColumn } =
                    to_proto::text_range(d.range, &line_index);
                Diagnostic {
                    message: d.message,
                    severity: to_proto::severity(d.severity),
                    startLineNumber,
                    startColumn,
                    endLineNumber,
                    endColumn,
                }
            })
            .collect();

        serde_wasm_bindgen::to_value(&UpdateResult { diagnostics, highlights }).unwrap()
    }

    pub fn completions(&self, line_number: u32, column: u32) -> JsValue {
        const COMPLETION_CONFIG: CompletionConfig = CompletionConfig {
            enable_postfix_completions: true,
            enable_imports_on_the_fly: true,
            add_call_parenthesis: true,
            add_call_argument_snippets: true,
            snippet_cap: SnippetCap::new(true),
            insert_use: InsertUseConfig {
                merge: Some(MergeBehavior::Full),
                prefix_kind: PrefixKind::Plain,
                group: true,
            },
        };

        log::warn!("completions");
        let line_index = self.analysis.file_line_index(self.file_id).unwrap();

        let pos = file_position(line_number, column, &line_index, self.file_id);
        let res = match self.analysis.completions(&COMPLETION_CONFIG, pos).unwrap() {
            Some(items) => items,
            None => return JsValue::NULL,
        };

        let items: Vec<_> =
            res.into_iter().map(|item| to_proto::completion_item(item, &line_index)).collect();
        serde_wasm_bindgen::to_value(&items).unwrap()
    }

    pub fn hover(&self, line_number: u32, column: u32) -> JsValue {
        log::warn!("hover");
        let line_index = self.analysis.file_line_index(self.file_id).unwrap();

        let pos = file_position(line_number, column, &line_index, self.file_id);
        let info = match self.analysis.hover(pos, true, true).unwrap() {
            Some(info) => info,
            _ => return JsValue::NULL,
        };

        let value = info.info.markup.to_string();
        let hover = Hover {
            contents: vec![MarkdownString { value }],
            range: to_proto::text_range(info.range, &line_index),
        };

        serde_wasm_bindgen::to_value(&hover).unwrap()
    }

    pub fn code_lenses(&self) -> JsValue {
        log::warn!("code_lenses");
        let line_index = self.analysis.file_line_index(self.file_id).unwrap();

        let results: Vec<_> = self
            .analysis
            .file_structure(self.file_id)
            .unwrap()
            .into_iter()
            .filter(|it| match it.kind {
                ide::StructureNodeKind::SymbolKind(it) => match it {
                    ide_db::SymbolKind::Trait
                    | ide_db::SymbolKind::Struct
                    | ide_db::SymbolKind::Enum => true,
                    _ => false,
                },
                ide::StructureNodeKind::Region => true,
            })
            .filter_map(|it| {
                let position =
                    FilePosition { file_id: self.file_id, offset: it.node_range.start() };
                let nav_info = self.analysis.goto_implementation(position).unwrap()?;

                let title = if nav_info.info.len() == 1 {
                    "1 implementation".into()
                } else {
                    format!("{} implementations", nav_info.info.len())
                };

                let positions = nav_info
                    .info
                    .iter()
                    .map(|target| target.focus_range.unwrap_or_else(|| target.full_range))
                    .map(|range| to_proto::text_range(range, &line_index))
                    .collect();

                Some(CodeLensSymbol {
                    range: to_proto::text_range(it.node_range, &line_index),
                    command: Some(Command {
                        id: "editor.action.showReferences".into(),
                        title,
                        positions,
                    }),
                })
            })
            .collect();

        serde_wasm_bindgen::to_value(&results).unwrap()
    }

    pub fn references(&self, line_number: u32, column: u32, include_declaration: bool) -> JsValue {
        log::warn!("references");
        let line_index = self.analysis.file_line_index(self.file_id).unwrap();

        let pos = file_position(line_number, column, &line_index, self.file_id);
        let info = match self.analysis.find_all_refs(pos, None).unwrap() {
            Some(info) => info,
            _ => return JsValue::NULL,
        };

        let mut res = vec![];
        if include_declaration {
            if let Some(r) = info.declaration {
                let r = r.nav.focus_range.unwrap_or_else(|| r.nav.full_range);
                res.push(Highlight { tag: None, range: to_proto::text_range(r, &line_index) });
            }
        }
        info.references.iter().for_each(|(_id, ranges)| {
            // FIXME: handle multiple files
            for (r, _) in ranges {
                res.push(Highlight { tag: None, range: to_proto::text_range(*r, &line_index) });
            }
        });

        serde_wasm_bindgen::to_value(&res).unwrap()
    }

    pub fn prepare_rename(&self, line_number: u32, column: u32) -> JsValue {
        log::warn!("prepare_rename");
        let line_index = self.analysis.file_line_index(self.file_id).unwrap();

        let pos = file_position(line_number, column, &line_index, self.file_id);
        let refs = match self.analysis.find_all_refs(pos, None).unwrap() {
            None => return JsValue::NULL,
            Some(refs) => refs,
        };

        let declaration = refs.declaration.unwrap();
        let range = to_proto::text_range(declaration.nav.full_range, &line_index);
        let text = declaration.nav.name.to_string();

        serde_wasm_bindgen::to_value(&RenameLocation { range, text }).unwrap()
    }

    pub fn rename(&self, line_number: u32, column: u32, new_name: &str) -> JsValue {
        log::warn!("rename");
        let line_index = self.analysis.file_line_index(self.file_id).unwrap();

        let pos = file_position(line_number, column, &line_index, self.file_id);
        let change = match self.analysis.rename(pos, new_name).unwrap() {
            Ok(change) => change,
            Err(_) => return JsValue::NULL,
        };

        let result: Vec<_> = change
            .source_file_edits
            .iter()
            .flat_map(|(_, edit)| edit.iter())
            .map(|atom: &Indel| to_proto::text_edit(atom, &line_index))
            .collect();

        serde_wasm_bindgen::to_value(&result).unwrap()
    }

    pub fn signature_help(&self, line_number: u32, column: u32) -> JsValue {
        log::warn!("signature_help");
        let line_index = self.analysis.file_line_index(self.file_id).unwrap();

        let pos = file_position(line_number, column, &line_index, self.file_id);
        let call_info = match self.analysis.call_info(pos) {
            Ok(Some(call_info)) => call_info,
            _ => return JsValue::NULL,
        };

        let active_parameter = call_info.active_parameter;
        let sig_info = to_proto::signature_information(call_info);

        let result = SignatureHelp {
            signatures: [sig_info],
            activeSignature: 0,
            activeParameter: active_parameter,
        };
        serde_wasm_bindgen::to_value(&result).unwrap()
    }

    pub fn definition(&self, line_number: u32, column: u32) -> JsValue {
        log::warn!("definition");
        let line_index = self.analysis.file_line_index(self.file_id).unwrap();

        let pos = file_position(line_number, column, &line_index, self.file_id);
        let nav_info = match self.analysis.goto_definition(pos) {
            Ok(Some(nav_info)) => nav_info,
            _ => return JsValue::NULL,
        };

        let res = to_proto::location_links(nav_info, &line_index);
        serde_wasm_bindgen::to_value(&res).unwrap()
    }

    pub fn type_definition(&self, line_number: u32, column: u32) -> JsValue {
        log::warn!("type_definition");
        let line_index = self.analysis.file_line_index(self.file_id).unwrap();

        let pos = file_position(line_number, column, &line_index, self.file_id);
        let nav_info = match self.analysis.goto_type_definition(pos) {
            Ok(Some(nav_info)) => nav_info,
            _ => return JsValue::NULL,
        };

        let res = to_proto::location_links(nav_info, &line_index);
        serde_wasm_bindgen::to_value(&res).unwrap()
    }

    pub fn document_symbols(&self) -> JsValue {
        log::warn!("document_symbols");
        let line_index = self.analysis.file_line_index(self.file_id).unwrap();

        let struct_nodes = match self.analysis.file_structure(self.file_id) {
            Ok(struct_nodes) => struct_nodes,
            _ => return JsValue::NULL,
        };
        let mut parents: Vec<(DocumentSymbol, Option<usize>)> = Vec::new();

        for symbol in struct_nodes {
            let doc_symbol = DocumentSymbol {
                name: symbol.label.clone(),
                detail: symbol.detail.unwrap_or(symbol.label),
                kind: to_proto::symbol_kind(symbol.kind),
                range: to_proto::text_range(symbol.node_range, &line_index),
                children: None,
                tags: [if symbol.deprecated { SymbolTag::Deprecated } else { SymbolTag::None }],
                containerName: None,
                selectionRange: to_proto::text_range(symbol.navigation_range, &line_index),
            };
            parents.push((doc_symbol, symbol.parent));
        }
        let mut res = Vec::new();
        while let Some((node, parent)) = parents.pop() {
            match parent {
                None => res.push(node),
                Some(i) => {
                    let children = &mut parents[i].0.children;
                    if children.is_none() {
                        *children = Some(Vec::new());
                    }
                    children.as_mut().unwrap().push(node);
                }
            }
        }

        serde_wasm_bindgen::to_value(&res).unwrap()
    }

    pub fn type_formatting(&self, line_number: u32, column: u32, ch: char) -> JsValue {
        log::warn!("type_formatting");
        let line_index = self.analysis.file_line_index(self.file_id).unwrap();

        let mut pos = file_position(line_number, column, &line_index, self.file_id);
        pos.offset -= TextSize::of('.');

        let edit = self.analysis.on_char_typed(pos, ch);

        let (_file, edit) = match edit {
            Ok(Some(it)) => it.source_file_edits.into_iter().next().unwrap(),
            _ => return JsValue::NULL,
        };

        let change: Vec<TextEdit> = to_proto::text_edits(edit, &line_index);
        serde_wasm_bindgen::to_value(&change).unwrap()
    }

    pub fn folding_ranges(&self) -> JsValue {
        log::warn!("folding_ranges");
        let line_index = self.analysis.file_line_index(self.file_id).unwrap();
        if let Ok(folds) = self.analysis.folding_ranges(self.file_id) {
            let res: Vec<_> =
                folds.into_iter().map(|fold| to_proto::folding_range(fold, &line_index)).collect();
            serde_wasm_bindgen::to_value(&res).unwrap()
        } else {
            JsValue::NULL
        }
    }

    pub fn goto_implementation(&self, line_number: u32, column: u32) -> JsValue {
        log::warn!("goto_implementation");
        let line_index = self.analysis.file_line_index(self.file_id).unwrap();

        let pos = file_position(line_number, column, &line_index, self.file_id);
        let nav_info = match self.analysis.goto_implementation(pos) {
            Ok(Some(it)) => it,
            _ => return JsValue::NULL,
        };
        let res = to_proto::location_links(nav_info, &line_index);
        serde_wasm_bindgen::to_value(&res).unwrap()
    }
}

fn file_position(
    line_number: u32,
    column: u32,
    line_index: &ide::LineIndex,
    file_id: ide::FileId,
) -> ide::FilePosition {
    let line_col = ide::LineCol { line: line_number - 1, col: column - 1 };
    let offset = line_index.offset(line_col);
    ide::FilePosition { file_id, offset }
}
