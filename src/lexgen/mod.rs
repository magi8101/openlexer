//! Lexer generator module.
//!
//! Converts regex patterns to NFA (Thompson), then DFA (subset construction),
//! and generates lexer code in C, Java, or Python.

pub mod regex;
pub mod nfa;
pub mod dfa;
pub mod codegen;
pub mod rules;

pub use regex::{Regex, RegexAst, CharClass, CharRange};
pub use nfa::Nfa;
pub use dfa::Dfa;
pub use rules::{LexerSpec, LexerRule, RuleAction};
pub use codegen::TargetLanguage;

use crate::error::Result;

/// Parse a lexer specification from a string (.l file content)
pub fn parse_lexer_spec(input: &str) -> Result<LexerSpec> {
    LexerSpec::parse(input)
}

/// Generate lexer code from a specification
pub fn generate_code(spec: &LexerSpec, lang: &str) -> Result<String> {
    let target = match lang.to_lowercase().as_str() {
        "c" => TargetLanguage::C,
        "java" => TargetLanguage::Java,
        "python" | "py" => TargetLanguage::Python,
        _ => return Err(crate::error::Error::InvalidLanguage(lang.to_string())),
    };
    codegen::generate_lexer_from_spec_with_conditions(spec, target)
}
