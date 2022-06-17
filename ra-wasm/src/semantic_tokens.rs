//! Semantic Tokens helpers
#![allow(non_camel_case_types)]

use std::ops;

use crate::return_types;

#[repr(u8)]
#[allow(dead_code)]
pub(crate) enum SemanticTokenType {
    COMMENT,
    STRING,
    KEYWORD,
    NUMBER,
    REGEXP,
    OPERATOR,
    NAMESPACE,
    TYPE,
    STRUCT,
    CLASS,
    INTERFACE,
    ENUM,
    TYPE_PARAMETER,
    FUNCTION,
    MEMBER,
    MACRO,
    VARIABLE,
    PARAMETER,
    PROPERTY,
    LABEL,
    UNSUPPORTED,
}

macro_rules! define_semantic_token_modifiers {
    ($($ident:ident),*$(,)?) => {
        #[derive(PartialEq)]
        pub(crate) enum SemanticTokenModifier {
            $($ident),*
        }

        pub(crate) const SUPPORTED_MODIFIERS: &[SemanticTokenModifier] = &[
            $(SemanticTokenModifier::$ident),*
        ];
    };
}

define_semantic_token_modifiers![
    DOCUMENTATION,
    DECLARATION,
    DEFINITION,
    STATIC,
    ABSTRACT,
    DEPRECATED,
    READONLY,
    DEFAULT_LIBRARY,
    // custom
    ASYNC,
    ATTRIBUTE_MODIFIER,
    CALLABLE,
    CONSTANT,
    CONSUMING,
    CONTROL_FLOW,
    CRATE_ROOT,
    INJECTED,
    INTRA_DOC_LINK,
    LIBRARY,
    MUTABLE,
    PUBLIC,
    REFERENCE,
    TRAIT_MODIFIER,
    UNSAFE,
];

#[derive(Default)]
pub(crate) struct ModifierSet(pub(crate) u32);

impl ops::BitOrAssign<SemanticTokenModifier> for ModifierSet {
    fn bitor_assign(&mut self, rhs: SemanticTokenModifier) {
        let idx = SUPPORTED_MODIFIERS.iter().position(|it| it == &rhs).unwrap();
        self.0 |= 1 << idx;
    }
}

/// Tokens are encoded relative to each other.
///
/// This is a direct port of <https://github.com/microsoft/vscode-languageserver-node/blob/f425af9de46a0187adb78ec8a46b9b2ce80c5412/server/src/sematicTokens.proposed.ts#L45>
pub(crate) struct SemanticTokensBuilder {
    prev_line: u32,
    prev_char: u32,
    data: Vec<u32>,
}

impl SemanticTokensBuilder {
    pub(crate) fn new() -> Self {
        SemanticTokensBuilder { prev_line: 0, prev_char: 0, data: Vec::new() }
    }

    /// Push a new token onto the builder
    pub(crate) fn push(
        &mut self,
        range: return_types::Range,
        token_index: u32,
        modifier_bitset: u32,
    ) {
        let mut push_line = range.startLineNumber - 1;
        let mut push_char = range.startColumn - 1;

        if !self.data.is_empty() {
            push_line -= self.prev_line;
            if push_line == 0 {
                push_char -= self.prev_char;
            }
        }

        // A token cannot be multiline
        let token_len = range.endColumn - range.startColumn;

        let token = [push_line, push_char, token_len, token_index, modifier_bitset];

        self.data.extend_from_slice(&token);

        self.prev_line = range.startLineNumber - 1;
        self.prev_char = range.startColumn - 1;
    }

    pub(crate) fn build(self) -> Vec<u32> {
        self.data
    }
}

pub(crate) fn type_index(ty: SemanticTokenType) -> u32 {
    ty as u32
}
