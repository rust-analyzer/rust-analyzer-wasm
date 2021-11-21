//! Conversion of rust-analyzer specific types to return_types equivalents.
use crate::{return_types, semantic_tokens};

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
        kind: completion_item_kind(item.kind()),
        label: item.label().to_string(),
        range,
        detail: item.detail().map(|it| it.to_string()),
        insertText: text,
        insertTextRules: if item.is_snippet() {
            return_types::CompletionItemInsertTextRule::InsertAsSnippet
        } else {
            return_types::CompletionItemInsertTextRule::None
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
    let documentation = call_info.doc.as_ref().map(|it| markdown_string(it));

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
    let selection = text_range(nav_info.range, line_index);
    nav_info
        .info
        .into_iter()
        .map(|nav| {
            let range = text_range(nav.full_range, line_index);

            let target_selection_range =
                nav.focus_range.map(|it| text_range(it, line_index)).unwrap_or(range);

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
    let range = text_range(fold.range, ctx);
    return_types::FoldingRange {
        start: range.startLineNumber,
        end: range.endLineNumber,
        kind: match fold.kind {
            ide::FoldKind::Comment => Some(return_types::FoldingRangeKind::Comment),
            ide::FoldKind::Imports => Some(return_types::FoldingRangeKind::Imports),
            ide::FoldKind::Region => Some(return_types::FoldingRangeKind::Region),
            _ => None,
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

pub(crate) type SemanticTokens = Vec<u32>;

pub(crate) fn semantic_tokens(
    text: &str,
    line_index: &ide::LineIndex,
    highlights: Vec<ide::HlRange>,
) -> SemanticTokens {
    let mut builder = semantic_tokens::SemanticTokensBuilder::new();

    for highlight_range in highlights {
        if highlight_range.highlight.is_empty() {
            continue;
        }
        let (ty, mods) = semantic_token_type_and_modifiers(highlight_range.highlight);
        let token_index = semantic_tokens::type_index(ty);
        let modifier_bitset = mods.0;

        for mut text_range in line_index.lines(highlight_range.range) {
            if text[text_range].ends_with('\n') {
                text_range = ide::TextRange::new(
                    text_range.start(),
                    text_range.end() - ide::TextSize::of('\n'),
                );
            }
            let range = self::text_range(text_range, line_index);

            builder.push(range, token_index, modifier_bitset);
        }
    }

    builder.build()
}

fn semantic_token_type_and_modifiers(
    highlight: ide::Highlight,
) -> (semantic_tokens::SemanticTokenType, semantic_tokens::ModifierSet) {
    use ide::{HlMod, HlTag, SymbolKind};
    use semantic_tokens::*;
    let mut mods = ModifierSet::default();
    let type_ = match highlight.tag {
        HlTag::Symbol(symbol) => match symbol {
            SymbolKind::Module => SemanticTokenType::NAMESPACE,
            SymbolKind::Impl => SemanticTokenType::TYPE,
            SymbolKind::Field => SemanticTokenType::PROPERTY,
            SymbolKind::TypeParam => SemanticTokenType::TYPE_PARAMETER,
            SymbolKind::ConstParam => SemanticTokenType::PARAMETER,
            SymbolKind::LifetimeParam => SemanticTokenType::TYPE_PARAMETER,
            SymbolKind::Label => SemanticTokenType::LABEL,
            SymbolKind::ValueParam => SemanticTokenType::PARAMETER,
            SymbolKind::SelfParam => SemanticTokenType::KEYWORD,
            SymbolKind::Local => SemanticTokenType::VARIABLE,
            SymbolKind::Function => {
                if highlight.mods.contains(HlMod::Associated) {
                    SemanticTokenType::MEMBER
                } else {
                    SemanticTokenType::FUNCTION
                }
            }
            SymbolKind::Const => {
                mods |= SemanticTokenModifier::CONSTANT;
                mods |= SemanticTokenModifier::STATIC;
                SemanticTokenType::VARIABLE
            }
            SymbolKind::Static => {
                mods |= SemanticTokenModifier::STATIC;
                SemanticTokenType::VARIABLE
            }
            SymbolKind::Struct => SemanticTokenType::TYPE,
            SymbolKind::Enum => SemanticTokenType::TYPE,
            SymbolKind::Variant => SemanticTokenType::MEMBER,
            SymbolKind::Union => SemanticTokenType::TYPE,
            SymbolKind::TypeAlias => SemanticTokenType::TYPE,
            SymbolKind::Trait => SemanticTokenType::INTERFACE,
            SymbolKind::Macro => SemanticTokenType::MACRO,
        },
        HlTag::Attribute => SemanticTokenType::UNSUPPORTED,
        HlTag::BoolLiteral => SemanticTokenType::NUMBER,
        HlTag::BuiltinAttr => SemanticTokenType::UNSUPPORTED,
        HlTag::BuiltinType => SemanticTokenType::TYPE,
        HlTag::ByteLiteral | HlTag::NumericLiteral => SemanticTokenType::NUMBER,
        HlTag::CharLiteral => SemanticTokenType::STRING,
        HlTag::Comment => SemanticTokenType::COMMENT,
        HlTag::EscapeSequence => SemanticTokenType::NUMBER,
        HlTag::FormatSpecifier => SemanticTokenType::MACRO,
        HlTag::Keyword => SemanticTokenType::KEYWORD,
        HlTag::None => SemanticTokenType::UNSUPPORTED,
        HlTag::Operator(_op) => SemanticTokenType::OPERATOR,
        HlTag::StringLiteral => SemanticTokenType::STRING,
        HlTag::UnresolvedReference => SemanticTokenType::UNSUPPORTED,
        HlTag::Punctuation(_punct) => SemanticTokenType::OPERATOR,
    };

    for modifier in highlight.mods.iter() {
        let modifier = match modifier {
            HlMod::Associated => continue,
            HlMod::Async => SemanticTokenModifier::ASYNC,
            HlMod::Attribute => SemanticTokenModifier::ATTRIBUTE_MODIFIER,
            HlMod::Callable => SemanticTokenModifier::CALLABLE,
            HlMod::Consuming => SemanticTokenModifier::CONSUMING,
            HlMod::ControlFlow => SemanticTokenModifier::CONTROL_FLOW,
            HlMod::CrateRoot => SemanticTokenModifier::CRATE_ROOT,
            HlMod::DefaultLibrary => SemanticTokenModifier::DEFAULT_LIBRARY,
            HlMod::Definition => SemanticTokenModifier::DECLARATION,
            HlMod::Documentation => SemanticTokenModifier::DOCUMENTATION,
            HlMod::Injected => SemanticTokenModifier::INJECTED,
            HlMod::IntraDocLink => SemanticTokenModifier::INTRA_DOC_LINK,
            HlMod::Library => SemanticTokenModifier::LIBRARY,
            HlMod::Mutable => SemanticTokenModifier::MUTABLE,
            HlMod::Public => SemanticTokenModifier::PUBLIC,
            HlMod::Reference => SemanticTokenModifier::REFERENCE,
            HlMod::Static => SemanticTokenModifier::STATIC,
            HlMod::Trait => SemanticTokenModifier::TRAIT_MODIFIER,
            HlMod::Unsafe => SemanticTokenModifier::UNSAFE,
        };
        mods |= modifier;
    }

    (type_, mods)
}
