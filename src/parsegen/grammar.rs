//! Grammar AST and Parser.
//!
//! Parses Bison-style grammar files with semantic actions.
//!
//! Syntax:
//! ```text
//! %token NUMBER PLUS
//! %%
//! expr : expr PLUS term { $$ = $1 + $3; }
//!      | term           { $$ = $1; }
//!      ;
//! ```

use crate::error::{Error, Result};
use std::collections::HashMap;

/// A single member inside a %union declaration.
#[derive(Debug, Clone)]
pub struct UnionField {
    /// The C type of this member (e.g. "int", "double", "char *").
    pub c_type: String,
    /// The field name (e.g. "ival", "dval", "sval").
    pub name: String,
}

/// A %destructor declaration binding cleanup code to a type tag or symbol.
#[derive(Debug, Clone)]
pub struct Destructor {
    /// The code block to run (raw C/Java/Python).
    pub code: String,
    /// Which symbols or type tags this destructor applies to.
    /// "<ival>", "<*>", or a token/nonterminal name.
    pub targets: Vec<String>,
}

/// Abstract Syntax Tree for Grammar.
#[derive(Debug, Clone)]
pub struct Grammar {
    pub tokens: Vec<String>,
    pub start_symbol: String,
    pub rules: Vec<Rule>,
    pub precedence: Vec<Precedence>,
    /// %union { ... } declaration: list of typed fields.
    pub union_fields: Vec<UnionField>,
    /// Raw %union body for verbatim output (Bison-compatible).
    pub raw_union_body: Option<String>,
    /// Maps token names to their type tag from %token <tag> NAME.
    pub token_types: HashMap<String, String>,
    /// Maps nonterminal names to their type tag from %type <tag> sym or %nterm <tag> sym.
    pub nterm_types: HashMap<String, String>,
    /// Whether %glr-parser was specified.
    pub glr_mode: bool,
    /// Whether %locations was specified.
    pub locations: bool,
    /// %destructor declarations.
    pub destructors: Vec<Destructor>,
    /// Whether %define parse.error detailed (or verbose) was specified.
    pub error_verbose: bool,
    /// Whether %define parse.lac full was specified (LAC = Lookahead Correction).
    pub lac_enabled: bool,
    /// Prologue code from %{ ... %} section.
    pub prologue: Option<String>,
}

#[derive(Debug, Clone)]
pub struct Rule {
    pub lhs: String,
    pub rhs: Vec<Symbol>,
    pub action: Option<String>,
    pub precedence_sym: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Symbol {
    Terminal(String),
    NonTerminal(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Assoc {
    Left,
    Right,
    NonAssoc,
    /// Precedence only - no associativity defined.
    /// Using two operators with PrecedenceOnly and same precedence level
    /// that would require associativity is an unresolved conflict.
    PrecedenceOnly,
}

#[derive(Debug, Clone)]
pub struct Precedence {
    pub assoc: Assoc,
    pub symbols: Vec<String>,
}

impl Grammar {
    pub fn new() -> Self {
        Self {
            tokens: Vec::new(),
            start_symbol: String::new(),
            rules: Vec::new(),
            precedence: Vec::new(),
            union_fields: Vec::new(),
            raw_union_body: None,
            token_types: HashMap::new(),
            nterm_types: HashMap::new(),
            glr_mode: false,
            locations: false,
            destructors: Vec::new(),
            error_verbose: false,
            lac_enabled: false,
            prologue: None,
        }
    }

    /// Looks up the type tag for a given symbol (terminal or nonterminal).
    /// Returns None if no type was declared.
    pub fn symbol_type(&self, name: &str) -> Option<&String> {
        self.token_types
            .get(name)
            .or_else(|| self.nterm_types.get(name))
    }

    /// Returns true if a %union was declared.
    pub fn has_union(&self) -> bool {
        self.raw_union_body.is_some() || !self.union_fields.is_empty()
    }

    /// Parses a grammar string.
    pub fn parse(input: &str) -> Result<Self> {
        let mut parser = GrammarParser::new(input);
        parser.parse()
    }
}

struct GrammarParser<'a> {
    input: &'a str,
    pos: usize,
    grammar: Grammar,
}

impl<'a> GrammarParser<'a> {
    fn new(input: &'a str) -> Self {
        Self {
            input,
            pos: 0,
            grammar: Grammar::new(),
        }
    }

    fn parse(&mut self) -> Result<Grammar> {
        // First, check if this looks like a lexer specification (.l file) instead of grammar
        self.validate_not_lexer()?;

        // 0. Parse prologue %{ ... %}
        self.parse_prologue();

        // 1. Declarations section (before %%)
        self.parse_declarations()?;

        // 2. Separator
        self.skip_whitespace_and_comments();
        if self.consume("%%") {
            // 3. Rules section
            self.parse_rules()?;
        } else {
            return Err(Error::GrammarError {
                line: 0,
                message: "Missing %% separator".to_string(),
            });
        }

        // 4. Default start symbol if not specified
        if self.grammar.start_symbol.is_empty() {
            if let Some(first_rule) = self.grammar.rules.first() {
                self.grammar.start_symbol = first_rule.lhs.clone();
            }
        }

        Ok(self.grammar.clone())
    }

    fn parse_prologue(&mut self) {
        self.skip_whitespace_and_comments();
        if self.consume("%{") {
            // Capture content until %}
            let start = self.pos;
            while !self.is_eof() {
                if self.peek_str("%}") {
                    let content = self.input[start..self.pos].to_string();
                    self.grammar.prologue = Some(content);
                    self.consume("%}");
                    return;
                }
                self.advance();
            }
        }
    }

    fn skip_whitespace_and_comments(&mut self) {
        loop {
            // Skip whitespace
            while let Some(c) = self.input[self.pos..].chars().next() {
                if c.is_whitespace() {
                    self.pos += c.len_utf8();
                } else {
                    break;
                }
            }

            // Skip C-style comments /* ... */
            if self.consume("/*") {
                while !self.is_eof() {
                    if self.consume("*/") {
                        break;
                    }
                    self.advance();
                }
                continue;
            }

            // Skip C++ style comments // ...
            if self.consume("//") {
                while !self.is_eof() && self.peek_char() != '\n' {
                    self.advance();
                }
                continue;
            }

            break;
        }
    }

    fn parse_declarations(&mut self) -> Result<()> {
        loop {
            self.skip_whitespace_and_comments();
            if self.peek_str("%%") || self.is_eof() {
                break;
            }

            if self.consume("%token") {
                self.parse_token_decl()?;
            } else if self.consume("%left") {
                self.parse_prec(Assoc::Left)?;
            } else if self.consume("%right") {
                self.parse_prec(Assoc::Right)?;
            } else if self.consume("%nonassoc") {
                self.parse_prec(Assoc::NonAssoc)?;
            } else if self.consume("%precedence") {
                self.parse_prec(Assoc::PrecedenceOnly)?;
            } else if self.consume("%start") {
                self.skip_whitespace();
                let start = self.parse_ident()?;
                self.grammar.start_symbol = start;
            } else if self.consume("%union") {
                self.parse_union_decl()?;
            } else if self.consume("%type") {
                self.parse_type_decl(false)?;
            } else if self.consume("%nterm") {
                self.parse_type_decl(false)?;
            } else if self.consume("%glr-parser") {
                self.grammar.glr_mode = true;
            } else if self.consume("%locations") {
                self.grammar.locations = true;
            } else if self.consume("%destructor") {
                self.parse_destructor_decl()?;
            } else if self.consume("%define") {
                self.parse_define_decl()?;
            } else {
                // Ignore unknown decls or comments
                self.advance();
            }
        }
        Ok(())
    }

    fn parse_token_decl(&mut self) -> Result<()> {
        // Check for optional type tag: %token <type> NAME1 NAME2 ...
        self.skip_whitespace();
        let type_tag = if self.peek_char() == '<' {
            Some(self.parse_type_tag()?)
        } else {
            None
        };

        loop {
            self.skip_whitespace();
            if self.peek_char() == '%' || self.is_eof() || self.peek_char() == '\n' {
                break;
            }
            // Stop if we hit a newline-like boundary (next % directive)
            if self.peek_str("%") {
                break;
            }
            let name = self.parse_ident()?;
            if !name.is_empty() {
                self.grammar.tokens.push(name.clone());
                if let Some(ref tag) = type_tag {
                    self.grammar.token_types.insert(name, tag.clone());
                }
            } else {
                break;
            }
        }
        Ok(())
    }

    /// Parses a <type_tag> like <ival>, <dval>, etc.
    fn parse_type_tag(&mut self) -> Result<String> {
        if !self.consume("<") {
            return Err(Error::GrammarError {
                line: 0,
                message: "Expected '<' for type tag".to_string(),
            });
        }
        let start = self.pos;
        while !self.is_eof() && self.peek_char() != '>' {
            self.advance();
        }
        let tag = self.input[start..self.pos].trim().to_string();
        if !self.consume(">") {
            return Err(Error::GrammarError {
                line: 0,
                message: "Expected '>' to close type tag".to_string(),
            });
        }
        Ok(tag)
    }

    /// Parses %union { type1 name1; type2 name2; ... }
    fn parse_union_decl(&mut self) -> Result<()> {
        self.skip_whitespace_and_comments();
        if !self.consume("{") {
            return Err(Error::GrammarError {
                line: 0,
                message: "Expected '{' after %union".to_string(),
            });
        }

        // Read until matching '}', preserving the raw body verbatim (Bison behavior)
        let mut depth = 1;
        let start = self.pos;
        while depth > 0 && !self.is_eof() {
            let c = self.peek_char();
            if c == '{' {
                depth += 1;
            }
            if c == '}' {
                depth -= 1;
            }
            if depth > 0 {
                self.advance();
            }
        }
        let body = self.input[start..self.pos].to_string();
        self.advance(); // consume closing '}'

        // Store raw body for verbatim output in generated code
        self.grammar.raw_union_body = Some(body.clone());

        // Also parse field declarations for type resolution
        // This handles simple "type name;" declarations for $$ type resolution
        for line in body.lines() {
            let trimmed = line.trim();
            // Skip comments and empty lines
            if trimmed.is_empty() || trimmed.starts_with("//") {
                continue;
            }
            // Skip lines that are part of nested struct/union
            if trimmed.starts_with("struct") || trimmed.starts_with("union") {
                continue;
            }
            if trimmed.starts_with("}") {
                continue;
            }
            // Skip obvious struct member lines (inside nested struct)
            if trimmed.contains("start") || trimmed.contains("end") || trimmed.contains("step") {
                continue; // Skip struct { double start; ... } members
            }
            // Strip C-style comment from end if present
            let no_comment = if let Some(idx) = trimmed.find("/*") {
                trimmed[..idx].trim()
            } else {
                trimmed
            };
            // Strip trailing semicolon
            let clean = no_comment.trim_end_matches(';').trim();
            if clean.is_empty() {
                continue;
            }
            // Split into type and name: last word is the name, everything before is the type
            let parts: Vec<&str> = clean.split_whitespace().collect();
            if parts.len() >= 2 {
                let name = parts.last().unwrap().trim_start_matches('*').to_string();
                let last_raw = *parts.last().unwrap();
                let stars: String = last_raw.chars().take_while(|c| *c == '*').collect();
                let type_parts: Vec<&str> = parts[..parts.len() - 1].to_vec();
                let mut c_type = type_parts.join(" ");
                if !stars.is_empty() {
                    c_type.push_str(&format!(" {}", stars));
                }
                let c_type = c_type.trim().to_string();
                self.grammar.union_fields.push(UnionField { c_type, name });
            }
        }
        Ok(())
    }

    /// Parses %type <tag> sym1 sym2 ... or %nterm <tag> sym1 sym2 ...
    fn parse_type_decl(&mut self, _is_token: bool) -> Result<()> {
        self.skip_whitespace();
        let tag = if self.peek_char() == '<' {
            self.parse_type_tag()?
        } else {
            return Err(Error::GrammarError {
                line: 0,
                message: "Expected <type> after %type or %nterm".to_string(),
            });
        };

        loop {
            self.skip_whitespace();
            if self.peek_char() == '%' || self.is_eof() {
                break;
            }
            let name = self.parse_ident()?;
            if name.is_empty() {
                break;
            }
            self.grammar.nterm_types.insert(name, tag.clone());
        }
        Ok(())
    }

    /// Parses %destructor { code } <tag> or %destructor { code } symbol
    fn parse_destructor_decl(&mut self) -> Result<()> {
        self.skip_whitespace_and_comments();
        let code = if self.peek_char() == '{' {
            self.parse_action_block()?
        } else {
            return Err(Error::GrammarError {
                line: 0,
                message: "Expected '{' after %destructor".to_string(),
            });
        };

        let mut targets = Vec::new();
        loop {
            self.skip_whitespace();
            if self.peek_char() == '%' || self.is_eof() {
                break;
            }
            if self.peek_char() == '<' {
                let tag = self.parse_type_tag()?;
                targets.push(format!("<{}>", tag));
            } else {
                let name = self.parse_ident()?;
                if name.is_empty() {
                    break;
                }
                targets.push(name);
            }
        }

        if !targets.is_empty() {
            self.grammar.destructors.push(Destructor { code, targets });
        }
        Ok(())
    }

    /// Parses %define directives like: %define parse.error detailed
    fn parse_define_decl(&mut self) -> Result<()> {
        self.skip_whitespace();
        let key = self.parse_dotted_ident();
        self.skip_whitespace();
        let value = self.parse_ident().unwrap_or_default();

        match key.as_str() {
            "parse.error" => {
                if value == "detailed" || value == "verbose" {
                    self.grammar.error_verbose = true;
                }
            }
            "api.value.type" => {
                // Could handle "union" here in the future
            }
            "lr.type" => {
                if value == "glr" {
                    self.grammar.glr_mode = true;
                }
            }
            "parse.lac" => {
                if value == "full" {
                    self.grammar.lac_enabled = true;
                }
            }
            _ => {
                // Unknown %define key, ignore
            }
        }
        Ok(())
    }

    /// Parses a dotted identifier like "parse.error" or "api.value.type".
    fn parse_dotted_ident(&mut self) -> String {
        let mut result = String::new();
        loop {
            let start = self.pos;
            while let Some(c) = self.input[self.pos..].chars().next() {
                if c.is_alphanumeric() || c == '_' {
                    self.pos += c.len_utf8();
                } else {
                    break;
                }
            }
            result.push_str(&self.input[start..self.pos]);
            if self.peek_char() == '.' {
                result.push('.');
                self.advance();
            } else {
                break;
            }
        }
        result
    }

    fn parse_prec(&mut self, assoc: Assoc) -> Result<()> {
        let mut symbols = Vec::new();
        loop {
            self.skip_whitespace();
            if self.peek_char() == '%' || self.is_eof() {
                break;
            }
            let name = self.parse_ident()?;
            if !name.is_empty() {
                symbols.push(name);
            } else {
                break;
            }
        }
        self.grammar.precedence.push(Precedence {
            assoc,
            symbols: symbols.clone(),
        });
        // Also register as tokens
        for s in symbols {
            if !self.grammar.tokens.contains(&s) {
                self.grammar.tokens.push(s);
            }
        }
        Ok(())
    }

    fn parse_rules(&mut self) -> Result<()> {
        loop {
            self.skip_whitespace_and_comments();
            if self.is_eof() || self.peek_str("%%") {
                break;
            }

            // LHS : RHS | RHS ;
            let lhs = self.parse_ident()?;
            if lhs.is_empty() {
                break;
            } // EOF or error

            self.skip_whitespace_and_comments();
            if !self.consume(":") {
                return Err(Error::GrammarError {
                    line: 0,
                    message: format!("Expected ':' after rule '{}'", lhs),
                });
            }

            // Parse alternatives
            loop {
                // Use the new signature to capture rhs, action, and precedence
                let (rhs, action, prec) = self.parse_rhs()?;
                self.grammar.rules.push(Rule {
                    lhs: lhs.clone(),
                    rhs,
                    action,
                    precedence_sym: prec,
                });

                self.skip_whitespace_and_comments();
                if self.consume("|") {
                    continue;
                } else if self.consume(";") {
                    break;
                } else {
                    break;
                }
            }
        }
        Ok(())
    }

    fn parse_rhs(&mut self) -> Result<(Vec<Symbol>, Option<String>, Option<String>)> {
        let mut rhs = Vec::new();
        let mut action = None;
        let mut prec = None;

        loop {
            self.skip_whitespace_and_comments();

            // Check for %prec
            if self.consume("%prec") {
                self.skip_whitespace();
                prec = Some(self.parse_ident()?);
                // Continue to see if there is an action
                continue;
            }

            if self.peek_char() == '{' {
                action = Some(self.parse_action_block()?);
                break;
            }
            if self.peek_char() == '|' || self.peek_char() == ';' || self.is_eof() {
                break;
            }

            let name = self.parse_ident()?;
            // Fix: consume may return empty if at special char like %
            if name.is_empty() {
                break;
            }

            if self.grammar.tokens.contains(&name) {
                rhs.push(Symbol::Terminal(name));
            } else {
                rhs.push(Symbol::NonTerminal(name));
            }
        }
        Ok((rhs, action, prec))
    }

    fn parse_action_block(&mut self) -> Result<String> {
        // Consume { ... } coping with nested braces
        if !self.consume("{") {
            return Ok(String::new());
        }

        let mut depth = 1;
        let start = self.pos;

        while depth > 0 && !self.is_eof() {
            let c = self.peek_char();
            if c == '{' {
                depth += 1;
            }
            if c == '}' {
                depth -= 1;
            }
            self.advance();
        }

        let end = self.pos - 1; // Exclude closing }
        Ok(self.input[start..end].to_string())
    }

    // Helper functions
    fn peek_char(&self) -> char {
        self.input[self.pos..].chars().next().unwrap_or('\0')
    }

    fn peek_str(&self, s: &str) -> bool {
        self.input[self.pos..].starts_with(s)
    }

    fn advance(&mut self) {
        if let Some(c) = self.input[self.pos..].chars().next() {
            self.pos += c.len_utf8();
        }
    }

    fn consume(&mut self, s: &str) -> bool {
        if self.input[self.pos..].starts_with(s) {
            self.pos += s.len();
            return true;
        }
        false
    }

    fn skip_whitespace(&mut self) {
        while let Some(c) = self.input[self.pos..].chars().next() {
            if c.is_whitespace() {
                self.pos += c.len_utf8();
            } else {
                break;
            }
        }
    }

    fn is_eof(&self) -> bool {
        self.pos >= self.input.len()
    }

    fn parse_ident(&mut self) -> Result<String> {
        self.skip_whitespace();
        let start = self.pos;
        while let Some(c) = self.input[self.pos..].chars().next() {
            if c.is_alphanumeric() || c == '_' {
                self.pos += c.len_utf8();
            } else {
                break;
            }
        }
        Ok(self.input[start..self.pos].to_string())
    }

    /// Validates that the input is not a lexer specification (.l file).
    /// Detects common patterns that indicate a lexer file was pasted instead of grammar rules.
    fn validate_not_lexer(&self) -> Result<()> {
        let mut lexer_indicators = 0;
        let mut indicator_examples = Vec::new();

        for line in self.input.lines() {
            let trimmed = line.trim();

            // Skip empty lines and comments
            if trimmed.is_empty() || trimmed.starts_with("//") || trimmed.starts_with("/*") {
                continue;
            }

            // Check for regex character classes like [a-zA-Z], [0-9]+
            if (trimmed.contains("[a-z")
                || trimmed.contains("[A-Z")
                || trimmed.contains("[0-9")
                || trimmed.contains("\\s")
                || trimmed.contains("\\d")
                || trimmed.contains("\\w"))
                && !trimmed.starts_with("%")
            {
                lexer_indicators += 2;
                if indicator_examples.len() < 2 {
                    indicator_examples.push(format!(
                        "'{}' (regex pattern)",
                        trimmed.chars().take(50).collect::<String>()
                    ));
                }
            }

            // Check for return TOKEN patterns: { return SOMETHING; }
            if trimmed.contains("return ")
                && trimmed.contains(";")
                && (trimmed.contains("{") || line.ends_with("}"))
            {
                lexer_indicators += 2;
                if indicator_examples.len() < 2 {
                    indicator_examples.push(format!(
                        "'{}' (lexer action)",
                        trimmed.chars().take(50).collect::<String>()
                    ));
                }
            }

            // Check for quoted literal patterns followed by action
            if (trimmed.starts_with('"') || trimmed.starts_with("'")) && trimmed.contains('{') {
                lexer_indicators += 1;
                if indicator_examples.len() < 2 {
                    indicator_examples.push(format!(
                        "'{}' (literal pattern with action)",
                        trimmed.chars().take(40).collect::<String>()
                    ));
                }
            }

            // Check for %s or %x (start conditions - lexer specific)
            if trimmed.starts_with("%s ") || trimmed.starts_with("%x ") {
                lexer_indicators += 3;
                if indicator_examples.len() < 2 {
                    indicator_examples.push(format!("'{}' (start condition)", trimmed));
                }
            }

            // Check for skip action (lexer specific)
            if trimmed.ends_with(" skip") || trimmed.ends_with("\tskip") {
                lexer_indicators += 2;
                if indicator_examples.len() < 2 {
                    indicator_examples.push(format!(
                        "'{}' (skip action)",
                        trimmed.chars().take(40).collect::<String>()
                    ));
                }
            }

            // Check for regex quantifiers like + * ? at end of pattern
            if (trimmed.contains("]+")
                || trimmed.contains(")*")
                || trimmed.contains(")?")
                || trimmed.contains(".+")
                || trimmed.contains(".*"))
                && !trimmed.starts_with("%")
                && !trimmed.starts_with("|")
            {
                lexer_indicators += 1;
            }
        }

        // If we found strong evidence of lexer syntax, return an error
        if lexer_indicators >= 5 {
            let examples_str = if indicator_examples.is_empty() {
                String::new()
            } else {
                format!(
                    "\n\nDetected lexer patterns:\n  - {}",
                    indicator_examples.join("\n  - ")
                )
            };

            return Err(Error::GrammarError {
                line: 0,
                message: format!(
                    "This appears to be a lexer specification (.l file), not a parser grammar (.y file).\n\n\
                    Grammar rules should have productions like:\n  \
                    expr: expr '+' term\n      \
                        | term\n      \
                        ;\n\n\
                    Please use the Lexer tab for lexer specifications.{}", 
                    examples_str
                ),
            });
        }

        Ok(())
    }
}
