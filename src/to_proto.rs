//! Conversion of rust-analyzer specific types to return_types equivalents.
use crate::return_types;

pub(crate) fn text_range(
    range: ide::TextRange,
    line_index: &ide::LineIndex,
) -> return_types::Range {
    let start = line_index.line_col(range.start());
    let end = line_index.line_col(range.end());

    return_types::Range {
        startLineNumber: start.line + 1,
        startColumn: start.col + 1,
        endLineNumber: end.line + 1,
        endColumn: end.col + 1,
    }
}

pub(crate) fn completion_item_kind(
    kind: ide::CompletionItemKind,
) -> return_types::CompletionItemKind {
    use return_types::CompletionItemKind::*;
    match kind {
        ide::CompletionItemKind::Keyword => Keyword,
        ide::CompletionItemKind::Snippet => Snippet,

        ide::CompletionItemKind::BuiltinType => Struct,
        ide::CompletionItemKind::Binding => Variable,
        ide::CompletionItemKind::SymbolKind(it) => match it {
            ide::SymbolKind::Const => Constant,
            ide::SymbolKind::ConstParam => Constant,
            ide::SymbolKind::Enum => Enum,
            ide::SymbolKind::Field => Field,
            ide::SymbolKind::Function => Function,
            ide::SymbolKind::Impl => Interface,
            ide::SymbolKind::Label => Constant,
            ide::SymbolKind::LifetimeParam => TypeParameter,
            ide::SymbolKind::Local => Variable,
            ide::SymbolKind::Macro => Function,
            ide::SymbolKind::Module => Module,
            ide::SymbolKind::SelfParam => Value,
            ide::SymbolKind::Static => Value,
            ide::SymbolKind::Struct => Struct,
            ide::SymbolKind::Trait => Interface,
            ide::SymbolKind::TypeAlias => Value,
            ide::SymbolKind::TypeParam => TypeParameter,
            ide::SymbolKind::Union => Struct,
            ide::SymbolKind::ValueParam => TypeParameter,
            ide::SymbolKind::Variant => User,
        },
        ide::CompletionItemKind::Method => Method,
        ide::CompletionItemKind::Attribute => Property,
        ide::CompletionItemKind::UnresolvedReference => User,
    }
}

pub(crate) fn severity(s: ide::Severity) -> return_types::MarkerSeverity {
    match s {
        ide::Severity::Error => return_types::MarkerSeverity::Error,
        ide::Severity::WeakWarning => return_types::MarkerSeverity::Hint,
    }
}

pub(crate) fn text_edit(indel: &ide::Indel, line_index: &ide::LineIndex) -> return_types::TextEdit {
    let text = indel.insert.clone();
    return_types::TextEdit { range: text_range(indel.delete, line_index), text }
}

pub(crate) fn text_edits(edit: ide::TextEdit, ctx: &ide::LineIndex) -> Vec<return_types::TextEdit> {
    edit.iter().map(|atom| text_edit(atom, ctx)).collect()
}

pub(crate) fn completion_item(
    item: ide::CompletionItem,
    line_index: &ide::LineIndex,
) -> return_types::CompletionItem {
    let mut additional_text_edits = Vec::new();
    let mut edit = None;
    // LSP does not allow arbitrary edits in completion, so we have to do a
    // non-trivial mapping here.
    for atom_edit in item.text_edit().iter() {
        if item.source_range().contains_range(atom_edit.delete) {
            edit = Some(if atom_edit.delete == item.source_range() {
                text_edit(atom_edit, line_index)
            } else {
                assert!(item.source_range().end() == atom_edit.delete.end());
                let range1 =
                    ide::TextRange::new(atom_edit.delete.start(), item.source_range().start());
                let range2 = item.source_range();
                let edit1 = ide::Indel::replace(range1, String::new());
                let edit2 = ide::Indel::replace(range2, atom_edit.insert.clone());
                additional_text_edits.push(text_edit(&edit1, line_index));
                text_edit(&edit2, line_index)
            })
        } else {
            edit = Some(text_edit(atom_edit, line_index));
        }
    }
    let return_types::TextEdit { range, text } = edit.unwrap();

    return_types::CompletionItem {
        kind: completion_item_kind(
            item.kind().unwrap_or(ide::CompletionItemKind::SymbolKind(ide::SymbolKind::Struct)),
        ),
        label: item.label().to_string(),
        range,
        detail: item.detail().map(|it| it.to_string()),
        insertText: text,
        insertTextRules: match item.insert_text_format() {
            ide::InsertTextFormat::PlainText => return_types::CompletionItemInsertTextRule::None,
            ide::InsertTextFormat::Snippet => {
                return_types::CompletionItemInsertTextRule::InsertAsSnippet
            }
        },
        documentation: item.documentation().map(|doc| markdown_string(doc.as_str())),
        filterText: item.lookup().to_string(),
        additionalTextEdits: additional_text_edits,
    }
}

pub(crate) fn signature_information(
    call_info: ide::CallInfo,
) -> return_types::SignatureInformation {
    use return_types::{ParameterInformation, SignatureInformation};

    let label = call_info.signature.clone();
    let documentation = call_info.doc.as_ref().map(|it| markdown_string(&it));

    let parameters: Vec<ParameterInformation> = call_info
        .parameter_labels()
        .into_iter()
        .map(|param| ParameterInformation { label: param.to_string() })
        .collect();

    SignatureInformation { label, documentation, parameters }
}

pub(crate) fn location_links(
    nav_info: ide::RangeInfo<Vec<ide::NavigationTarget>>,
    line_index: &ide::LineIndex,
) -> Vec<return_types::LocationLink> {
    let selection = text_range(nav_info.range, &line_index);
    nav_info
        .info
        .into_iter()
        .map(|nav| {
            let range = text_range(nav.full_range, &line_index);

            let target_selection_range =
                nav.focus_range.map(|it| text_range(it, &line_index)).unwrap_or(range);

            return_types::LocationLink {
                originSelectionRange: selection,
                range,
                targetSelectionRange: target_selection_range,
            }
        })
        .collect()
}

pub(crate) fn symbol_kind(kind: ide::StructureNodeKind) -> return_types::SymbolKind {
    use return_types::SymbolKind;

    let kind = match kind {
        ide::StructureNodeKind::SymbolKind(it) => it,
        ide::StructureNodeKind::Region => return SymbolKind::Property,
    };

    match kind {
        ide::SymbolKind::Const => SymbolKind::Constant,
        ide::SymbolKind::ConstParam => SymbolKind::Constant,
        ide::SymbolKind::Enum => SymbolKind::Enum,
        ide::SymbolKind::Field => SymbolKind::Field,
        ide::SymbolKind::Function => SymbolKind::Function,
        ide::SymbolKind::Impl => SymbolKind::Interface,
        ide::SymbolKind::Label => SymbolKind::Constant,
        ide::SymbolKind::LifetimeParam => SymbolKind::TypeParameter,
        ide::SymbolKind::Local => SymbolKind::Variable,
        ide::SymbolKind::Macro => SymbolKind::Function,
        ide::SymbolKind::Module => SymbolKind::Module,
        ide::SymbolKind::SelfParam => SymbolKind::Variable,
        ide::SymbolKind::Static => SymbolKind::Constant,
        ide::SymbolKind::Struct => SymbolKind::Struct,
        ide::SymbolKind::Trait => SymbolKind::Interface,
        ide::SymbolKind::TypeAlias => SymbolKind::TypeParameter,
        ide::SymbolKind::TypeParam => SymbolKind::TypeParameter,
        ide::SymbolKind::Union => SymbolKind::Struct,
        ide::SymbolKind::ValueParam => SymbolKind::TypeParameter,
        ide::SymbolKind::Variant => SymbolKind::EnumMember,
    }
}

pub(crate) fn folding_range(fold: ide::Fold, ctx: &ide::LineIndex) -> return_types::FoldingRange {
    let range = text_range(fold.range, &ctx);
    return_types::FoldingRange {
        start: range.startLineNumber,
        end: range.endLineNumber,
        kind: match fold.kind {
            ide::FoldKind::Comment => Some(return_types::FoldingRangeKind::Comment),
            ide::FoldKind::Imports => Some(return_types::FoldingRangeKind::Imports),
            ide::FoldKind::Mods => None,
            ide::FoldKind::Block => None,
            ide::FoldKind::ArgList => None,
            ide::FoldKind::Region => Some(return_types::FoldingRangeKind::Region),
        },
    }
}

fn markdown_string(s: &str) -> return_types::MarkdownString {
    fn code_line_ignored_by_rustdoc(line: &str) -> bool {
        let trimmed = line.trim();
        trimmed == "#" || trimmed.starts_with("# ") || trimmed.starts_with("#\t")
    }

    let mut processed_lines = Vec::new();
    let mut in_code_block = false;
    for line in s.lines() {
        if in_code_block && code_line_ignored_by_rustdoc(line) {
            continue;
        }

        if line.starts_with("```") {
            in_code_block ^= true
        }

        let line = if in_code_block && line.starts_with("```") && !line.contains("rust") {
            "```rust"
        } else {
            line
        };

        processed_lines.push(line);
    }

    return_types::MarkdownString { value: processed_lines.join("\n") }
}
