pub mod action_parser;
pub mod codegen;
pub mod error_recovery;
pub mod first;
pub mod glr;
pub mod grammar;
pub mod lalr;
pub mod multiline_handler;
pub mod printf_converter;

pub use glr::{GlrParser, GlrTable, ParseForest, Token as GlrToken};
pub use grammar::{Grammar, Rule};
pub use lalr::ParsingTable;

use crate::error::Result;
use crate::lexgen::TargetLanguage;

/// Parse a grammar specification from a string (.y file content)
pub fn parse_grammar(input: &str) -> Result<Grammar> {
    Grammar::parse(input)
}

/// Generate parser code from a grammar
pub fn generate_code(grammar: &Grammar, lang: &str) -> Result<String> {
    let target = match lang.to_lowercase().as_str() {
        "c" => TargetLanguage::C,
        "java" => TargetLanguage::Java,
        "python" | "py" => TargetLanguage::Python,
        _ => return Err(crate::error::Error::InvalidLanguage(lang.to_string())),
    };
    let table = ParsingTable::build(grammar)?;
    codegen::generate_parser(&table, grammar, target)
}
