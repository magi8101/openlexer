//! OpenLexer - Flex/Bison Replacement
//!
//! A lexer and parser generator that produces C, Java, or Python code
//! from user-defined token patterns and grammar rules.
//!
//! ## Usage
//!
//! ```bash
//! openlexer --lexer rules.l --parser grammar.y --lang c -o output/
//! ```

pub mod error;
pub mod lexgen;
pub mod parsegen;

pub use error::{Error, Result};
