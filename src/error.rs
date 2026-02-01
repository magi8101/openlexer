//! Error types for OpenLexer.

use thiserror::Error;

/// Result type alias.
pub type Result<T> = std::result::Result<T, Error>;

/// Errors that can occur in OpenLexer.
#[derive(Debug, Error)]
pub enum Error {
    /// Invalid regex pattern
    #[error("Invalid regex pattern at position {position}: {message}")]
    RegexError { position: usize, message: String },

    /// Invalid lexer specification file
    #[error("Lexer spec error at line {line}: {message}")]
    LexerSpecError { line: usize, message: String },

    /// Invalid grammar rule
    #[error("Invalid grammar rule at line {line}: {message}")]
    GrammarError { line: usize, message: String },

    /// NFA construction error
    #[error("NFA construction failed: {0}")]
    NfaError(String),

    /// DFA construction error
    #[error("DFA construction failed: {0}")]
    DfaError(String),

    /// LALR table construction error
    #[error("LALR table construction failed: {0}")]
    LalrError(String),

    /// Code generation error
    #[error("Code generation failed: {0}")]
    CodegenError(String),

    /// File I/O error
    #[error("File error: {0}")]
    IoError(#[from] std::io::Error),

    /// Invalid target language
    #[error("Invalid target language: {0}")]
    InvalidLanguage(String),
}
