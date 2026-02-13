//! Lexer generator module.
//!
//! Converts regex patterns to NFA (Thompson), then DFA (subset construction),
//! and generates lexer code in C, Java, or Python.

pub mod codegen;
pub mod dfa;
pub mod nfa;
pub mod regex;
pub mod rules;
pub mod unicode;

pub use codegen::TargetLanguage;
pub use dfa::Dfa;
pub use nfa::Nfa;
pub use regex::{CharClass, CharRange, Regex, RegexAst};
pub use rules::{LexerRule, LexerSpec, RuleAction};

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

/// Generate a standalone test driver for the given language.
/// This can be used independently or is already embedded in generate_code output.
pub fn generate_test_driver(lang: &str) -> Result<String> {
    match lang.to_lowercase().as_str() {
        "c" => Ok(codegen::generate_c_test_driver()),
        "java" => Ok(codegen::generate_java_test_driver()),
        "python" | "py" => Ok(codegen::generate_python_test_driver()),
        _ => Err(crate::error::Error::InvalidLanguage(lang.to_string())),
    }
}
