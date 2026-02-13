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

#![allow(clippy::redundant_pattern_matching)]
#![allow(clippy::new_without_default)]
#![allow(clippy::single_char_add_str)]
#![allow(clippy::collapsible_else_if)]
#![allow(clippy::collapsible_match)]
#![allow(clippy::too_many_arguments)]
#![allow(clippy::type_complexity)]
#![allow(clippy::match_like_matches_macro)]
#![allow(clippy::if_same_then_else)]
#![allow(clippy::should_implement_trait)]
#![allow(clippy::len_without_is_empty)]
#![allow(clippy::useless_format)]
#![allow(clippy::doc_lazy_continuation)]
#![allow(clippy::manual_strip)]

pub mod error;
pub mod lexgen;
pub mod parsegen;

pub use error::{Error, Result};
