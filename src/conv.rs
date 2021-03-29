use super::return_types;
use ide::{
    CallInfo, CompletionItem, CompletionItemKind, Documentation, FileId, FilePosition, Fold,
    FoldKind, Indel, InsertTextFormat, LineCol, LineIndex, NavigationTarget, RangeInfo, Severity,
    StructureNodeKind, TextRange, TextEdit
};

pub trait Conv {
    type Output;
    fn conv(self) -> Self::Output;
}

pub trait ConvWith<CTX> {
    type Output;
    fn conv_with(self, ctx: CTX) -> Self::Output;
}

#[derive(Clone, Copy)]
pub struct Position {
    pub line_number: u32,
    pub column: u32,
}

impl ConvWith<(&LineIndex, FileId)> for Position {
    type Output = FilePosition;

    fn conv_with(self, (line_index, file_id): (&LineIndex, FileId)) -> Self::Output {
        let line_col = LineCol { line: self.line_number - 1, col: self.column - 1 };
        let offset = line_index.offset(line_col);
        FilePosition { file_id, offset }
    }
}

impl ConvWith<&LineIndex> for TextRange {
    type Output = return_types::Range;

    fn conv_with(self, line_index: &LineIndex) -> Self::Output {
        let start = line_index.line_col(self.start());
        let end = line_index.line_col(self.end());

        return_types::Range {
            startLineNumber: start.line + 1,
            startColumn: start.col + 1,
            endLineNumber: end.line + 1,
            endColumn: end.col + 1,
        }
    }
}

impl Conv for CompletionItemKind {
    type Output = return_types::CompletionItemKind;

    fn conv(self) -> Self::Output {
        use return_types::CompletionItemKind::*;
        match self {
            CompletionItemKind::Keyword => Keyword,
            CompletionItemKind::Snippet => Snippet,

            CompletionItemKind::BuiltinType => Struct,
            CompletionItemKind::Binding => Variable,
            CompletionItemKind::SymbolKind(it) => match it {
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
            CompletionItemKind::Method => Method,
            CompletionItemKind::Attribute => Property,
            CompletionItemKind::UnresolvedReference => User,
        }
    }
}

impl Conv for Severity {
    type Output = return_types::MarkerSeverity;

    fn conv(self) -> Self::Output {
        match self {
            Severity::Error => return_types::MarkerSeverity::Error,
            Severity::WeakWarning => return_types::MarkerSeverity::Hint,
        }
    }
}

impl ConvWith<&LineIndex> for &Indel {
    type Output = return_types::TextEdit;

    fn conv_with(self, line_index: &LineIndex) -> Self::Output {
        let text = self.insert.clone();
        return_types::TextEdit { range: self.delete.conv_with(line_index), text }
    }
}

impl ConvWith<&LineIndex> for CompletionItem {
    type Output = return_types::CompletionItem;

    fn conv_with(self, line_index: &LineIndex) -> Self::Output {
        let mut additional_text_edits = Vec::new();
        let mut text_edit = None;
        // LSP does not allow arbitrary edits in completion, so we have to do a
        // non-trivial mapping here.
        for atom_edit in self.text_edit().iter() {
            if self.source_range().contains_range(atom_edit.delete) {
                text_edit = Some(if atom_edit.delete == self.source_range() {
                    atom_edit.conv_with(line_index)
                } else {
                    assert!(self.source_range().end() == atom_edit.delete.end());
                    let range1 =
                        TextRange::new(atom_edit.delete.start(), self.source_range().start());
                    let range2 = self.source_range();
                    let edit1 = Indel::replace(range1, String::new());
                    let edit2 = Indel::replace(range2, atom_edit.insert.clone());
                    additional_text_edits.push(edit1.conv_with(line_index));
                    edit2.conv_with(line_index)
                })
            } else {
                text_edit = Some(atom_edit.conv_with(line_index));
            }
        }
        let return_types::TextEdit { range, text } = text_edit.unwrap();

        return_types::CompletionItem {
            kind: self
                .kind()
                .unwrap_or(CompletionItemKind::SymbolKind(ide::SymbolKind::Struct))
                .conv(),
            label: self.label().to_string(),
            range,
            detail: self.detail().map(|it| it.to_string()),
            insertText: text,
            insertTextRules: match self.insert_text_format() {
                InsertTextFormat::PlainText => return_types::CompletionItemInsertTextRule::None,
                InsertTextFormat::Snippet => {
                    return_types::CompletionItemInsertTextRule::InsertAsSnippet
                }
            },
            documentation: self.documentation().map(|doc| doc.conv()),
            filterText: self.lookup().to_string(),
            additionalTextEdits: additional_text_edits,
        }
    }
}

impl Conv for Documentation {
    type Output = return_types::MarkdownString;
    fn conv(self) -> Self::Output {
        conv_markdown_string(self.as_str())
    }
}

impl Conv for CallInfo {
    type Output = return_types::SignatureInformation;
    fn conv(self) -> Self::Output {
        use return_types::{ParameterInformation, SignatureInformation};

        let label = self.signature.clone();
        let documentation = self.doc.as_ref().map(|it| conv_markdown_string(&it));

        let parameters: Vec<ParameterInformation> = self
            .parameter_labels()
            .into_iter()
            .map(|param| ParameterInformation { label: param.to_string() })
            .collect();

        SignatureInformation { label, documentation, parameters }
    }
}

impl ConvWith<&LineIndex> for RangeInfo<Vec<NavigationTarget>> {
    type Output = Vec<return_types::LocationLink>;
    fn conv_with(self, line_index: &LineIndex) -> Self::Output {
        let selection = self.range.conv_with(&line_index);
        self.info
            .into_iter()
            .map(|nav| {
                let range = nav.full_range.conv_with(&line_index);

                let target_selection_range =
                    nav.focus_range.map(|it| it.conv_with(&line_index)).unwrap_or(range);

                return_types::LocationLink {
                    originSelectionRange: selection,
                    range,
                    targetSelectionRange: target_selection_range,
                }
            })
            .collect()
    }
}

impl Conv for ide::SymbolKind {
    type Output = return_types::SymbolKind;

    fn conv(self) -> Self::Output {
        use return_types::SymbolKind;
        match self {
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
}

impl Conv for StructureNodeKind {
    type Output = return_types::SymbolKind;

    fn conv(self) -> Self::Output {
        use return_types::SymbolKind;
        match self {
            StructureNodeKind::SymbolKind(it) => it.conv(),
            StructureNodeKind::Region => SymbolKind::Property,
        }
    }
}

impl ConvWith<&LineIndex> for TextEdit {
    type Output = Vec<return_types::TextEdit>;

    fn conv_with(self, ctx: &LineIndex) -> Self::Output {
        self.iter().map(|atom| atom.conv_with(ctx)).collect()
    }
}

impl ConvWith<&LineIndex> for Fold {
    type Output = return_types::FoldingRange;

    fn conv_with(self, ctx: &LineIndex) -> Self::Output {
        let range = self.range.conv_with(&ctx);
        return_types::FoldingRange {
            start: range.startLineNumber,
            end: range.endLineNumber,
            kind: match self.kind {
                FoldKind::Comment => Some(return_types::FoldingRangeKind::Comment),
                FoldKind::Imports => Some(return_types::FoldingRangeKind::Imports),
                FoldKind::Mods => None,
                FoldKind::Block => None,
                FoldKind::ArgList => None,
                FoldKind::Region => Some(return_types::FoldingRangeKind::Region),
            },
        }
    }
}


fn conv_markdown_string(s: &str) -> return_types::MarkdownString {
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
