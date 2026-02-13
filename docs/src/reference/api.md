# API Reference

This chapter documents the internal Rust API for programmatic use.

## Lexer Generator

### LexerSpec

Represents a parsed `.l` file.

```rust
pub struct LexerSpec {
    pub prologue: String,
    pub epilogue: String,
    pub definitions: Vec<(String, String)>,
    pub rules: Vec<LexerRule>,
    pub start_conditions: Vec<StartCondition>,
}

impl LexerSpec {
    /// Parse a lexer specification from source text
    pub fn parse(input: &str) -> Result<LexerSpec>;
}
```

### LexerRule

A single lexer rule (pattern + action).

```rust
pub struct LexerRule {
    pub pattern: String,
    pub action: RuleAction,
    pub conditions: Vec<String>,
}
```

### Code Generation

```rust
/// Generate lexer code from a specification
pub fn generate_code(spec: &LexerSpec, lang: &str) -> Result<String>;

/// Target languages
pub enum TargetLanguage {
    C,
    Java,
    Python,
}
```

## Parser Generator

### Grammar

Represents a parsed `.y` file.

```rust
pub struct Grammar {
    pub prologue: String,
    pub epilogue: String,
    pub tokens: Vec<String>,
    pub precedence: Vec<PrecedenceLevel>,
    pub rules: Vec<Rule>,
    pub start_symbol: Option<String>,
    pub union_def: Option<String>,
    pub glr_mode: bool,
}

impl Grammar {
    /// Parse a grammar specification from source text
    pub fn parse(input: &str) -> Result<Grammar>;
}
```

### Rule

A grammar production rule.

```rust
pub struct Rule {
    pub lhs: String,
    pub rhs: Vec<String>,
    pub action: Option<String>,
    pub precedence: Option<String>,
}
```

### ParsingTable

LALR(1) parsing tables.

```rust
pub struct ParsingTable {
    pub states: Vec<LrState>,
    pub action: HashMap<(usize, String), Action>,
    pub goto: HashMap<(usize, String), usize>,
    pub glr_conflict_actions: HashMap<usize, HashMap<String, Vec<Action>>>,
}

impl ParsingTable {
    /// Build parsing tables from a grammar
    pub fn build(grammar: &Grammar) -> Result<ParsingTable>;
}
```

### Action

Parser actions.

```rust
pub enum Action {
    Shift(usize),    // Shift and go to state
    Reduce(usize),   // Reduce by rule number
    Accept,          // Accept input
}
```

## GLR Parser

### GlrParser

Runtime GLR parser using Graph-Structured Stack.

```rust
pub struct GlrParser {
    table: GlrTable,
    gss: Gss,
    forest: ParseForest,
}

impl GlrParser {
    /// Create a new GLR parser from a parsing table
    pub fn new(table: GlrTable) -> GlrParser;
    
    /// Parse a token stream
    pub fn parse(&mut self, tokens: Vec<Token>) -> Result<Vec<ParseTree>>;
}
```

### Token

Input token for GLR parsing.

```rust
pub struct Token {
    pub kind: String,
    pub value: String,
    pub span: (usize, usize),
}
```

## Error Types

```rust
pub enum Error {
    /// Invalid regex syntax
    InvalidRegex(String),
    
    /// Grammar error
    GrammarError(String),
    
    /// Unsupported target language
    InvalidLanguage(String),
    
    /// Parse table construction failed
    TableError(String),
    
    /// I/O error
    IoError(std::io::Error),
}
```

## Example Usage

```rust
use openlexer::lexgen;
use openlexer::parsegen;

fn main() -> openlexer::Result<()> {
    // Generate lexer
    let lexer_spec = lexgen::parse_lexer_spec(include_str!("calc.l"))?;
    let lexer_code = lexgen::generate_code(&lexer_spec, "python")?;
    std::fs::write("lexer.py", lexer_code)?;
    
    // Generate parser
    let grammar = parsegen::parse_grammar(include_str!("calc.y"))?;
    let parser_code = parsegen::generate_code(&grammar, "python")?;
    std::fs::write("parser.py", parser_code)?;
    
    Ok(())
}
```
