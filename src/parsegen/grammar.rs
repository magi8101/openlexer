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
    /// Epilogue code from section after second %%.
    pub epilogue: Option<String>,
    /// Maps token names to their original literal strings (for textbook notation).
    /// E.g. "LPAREN" → "(", "IF" → "if". Used by generate_lexer_spec().
    pub token_literals: HashMap<String, String>,
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
            epilogue: None,
            token_literals: HashMap::new(),
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

    /// Generate a lexer specification (.l format) from the grammar's terminal symbols.
    /// This allows users to write just the grammar in textbook notation and get
    /// both a parser and a matching lexer automatically — like Bison+Flex but
    /// without needing to write a separate .l file.
    ///
    /// Terminal classification:
    ///   - Alphabetic tokens (e.g. "if", "else", "other") → keyword literals
    ///   - Single punctuation chars (e.g. "(", ")") → literal match
    ///   - ALL_CAPS identifiers → kept as-is (user-defined token names)
    pub fn generate_lexer_spec(&self) -> String {
        let mut rules = Vec::new();

        // Only include tokens that have explicit literals defined
        for token in &self.tokens {
            if let Some(literal) = self.token_literals.get(token) {
                // Escape regex metacharacters, but preserve escape sequences
                let escaped = Self::escape_lexer_pattern(literal);

                rules.push(format!(
                    "{}    {{ return {}; }}",
                    escaped, token
                ));
            }
        }

        // Add actual patterns for tokens without explicit literals
        // But skip precedence-only symbols (they shouldn't appear in lexer)
        let precedence_symbols = ["UMINUS", "PREC", "PPRIORITY"];
        let tokens_without_literals: Vec<_> = self.tokens
            .iter()
            .filter(|t| {
                !self.token_literals.contains_key(*t) &&
                !precedence_symbols.contains(&t.as_str())
            })
            .collect();

        if !tokens_without_literals.is_empty() {
            for token in tokens_without_literals {
                // Generate pattern based on token name
                let pattern = match token.as_str() {
                    // Numeric types
                    "NUMBER" | "NUM" | "INTEGER" | "INT" | "FLOAT" | "DOUBLE" | "DECIMAL" =>
                        "[0-9]+",
                    // Identifier types
                    "IDENTIFIER" | "ID" | "NAME" | "VAR" =>
                        "[a-zA-Z_][a-zA-Z0-9_]*",
                    // String literals
                    "STRING" | "STR" =>
                        "\"([^\"\\\\]|\\\\.)*\"",
                    // Comments
                    "COMMENT" =>
                        "//.*",
                    // Arithmetic operators
                    "PLUS"      => "\\+",
                    "MINUS"     => "-",
                    "STAR" | "TIMES" | "MUL" | "MULTIPLY" => "\\*",
                    "SLASH" | "DIVIDE" | "DIV" => "/",
                    "PERCENT" | "MOD" | "MODULO" => "%",
                    "CARET" | "POWER" | "POW"   => "\\^",
                    // Comparison / relational operators
                    "EQ" | "EQUAL" | "EQUALS"   => "==",
                    "NEQ" | "NOTEQUAL" | "NE"   => "!=",
                    "LTE" | "LE" | "LESSEQUAL"  => "<=",
                    "GTE" | "GE" | "GREATEREQUAL" => ">=",
                    "LT" | "LESS"               => "<",
                    "GT" | "GREATER"            => ">",
                    // Assignment
                    "ASSIGN"                    => "=",
                    // Logical operators
                    "AND" | "LAND"              => "&&",
                    "OR"  | "LOR"               => "\\|\\|",
                    "NOT" | "BANG"              => "!",
                    // Bitwise operators
                    "AMP" | "BITAND"            => "&",
                    "PIPE" | "BITOR"            => "\\|",
                    "TILDE"                     => "~",
                    // Delimiters / grouping
                    "LPAREN"                    => "\\(",
                    "RPAREN"                    => "\\)",
                    "LBRACKET" | "LBRACK"       => "\\[",
                    "RBRACKET" | "RBRACK"       => "\\]",
                    "LBRACE"                    => "\\{",
                    "RBRACE"                    => "\\}",
                    // Punctuation
                    "SEMICOLON" | "SEMI"        => ";",
                    "COLON"                     => ":",
                    "COMMA"                     => ",",
                    "DOT" | "PERIOD"            => "\\.",
                    "ARROW"                     => "->",
                    "DOUBLEARROW" | "FATARROW"   => "=>",
                    "DOUBLECOLON"               => "::",
                    "ELLIPSIS"                  => "\\.\\.\\.",
                    // Newline (explicit)
                    "NEWLINE" | "NL" | "EOL"    => "\\n",
                    // Prefix heuristics
                    _ if token.starts_with("NUM") => "[0-9]+",
                    _ if token.starts_with("ID") || token.starts_with("NAME") =>
                        "[a-zA-Z_][a-zA-Z0-9_]*",
                    // Fallback: treat as keyword-like identifier
                    _ => "[a-zA-Z_][a-zA-Z0-9_]*",
                };

                rules.push(format!(
                    "{}    {{ return {}; }}",
                    pattern, token
                ));
            }
        }

        // Always add whitespace skip and catch-all. Carefully avoid eating \n if it's explicitly a token.
        let has_newline_token = self.token_literals.values().any(|v| v == "\\n")
            || self.tokens.iter().any(|t| matches!(t.as_str(), "NEWLINE" | "NL" | "EOL"));
        let skip_rule = if has_newline_token {
            "[ \\t\\r]+"
        } else {
            "[ \\t\\r\\n]+"
        };
        rules.push(format!("{}    {{ /* skip whitespace */ }}", skip_rule));
        rules.push(".           { /* skip unknown */ }".to_string());

        format!(
            "/* Auto-generated lexer for grammar */\n\n%%\n\n{}\n\n%%\n",
            rules.join("\n")
        )
    }

    /// Escape a pattern for Lex/Flex lexer spec
    /// Preserves escape sequences like \n, \t but escapes regex metacharacters
    fn escape_lexer_pattern(s: &str) -> String {
        let mut result = String::new();
        let mut chars = s.chars().peekable();

        // Quote the literal if it contains special chars, otherwise quote it minimally
        result.push('"');

        while let Some(ch) = chars.next() {
            match ch {
                // Escape sequences - preserve them as-is
                '\\' if chars.peek() == Some(&'n') => {
                    result.push_str("\\n");
                    chars.next();
                }
                '\\' if chars.peek() == Some(&'t') => {
                    result.push_str("\\t");
                    chars.next();
                }
                '\\' if chars.peek() == Some(&'r') => {
                    result.push_str("\\r");
                    chars.next();
                }
                '\\' if chars.peek() == Some(&'\\') => {
                    result.push_str("\\\\");
                    chars.next();
                }
                // Regex metacharacters - escape them
                '(' | ')' | '[' | ']' | '{' | '}' | '.' | '*' | '+' | '?'
                | '^' | '$' | '|' => {
                    result.push('\\');
                    result.push(ch);
                }
                '"' => {
                    result.push('\\');
                    result.push('"');
                }
                _ => result.push(ch),
            }
        }

        result.push('"');
        result
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
    line: usize,
    grammar: Grammar,
    /// Pending mid-rule action rules to be added after current rule parsing.
    pending_mid_rule_actions: Vec<Rule>,
}

impl<'a> GrammarParser<'a> {
    fn new(input: &'a str) -> Self {
        Self {
            input,
            pos: 0,
            line: 1,
            grammar: Grammar::new(),
            pending_mid_rule_actions: Vec::new(),
        }
    }

    fn parse(&mut self) -> Result<Grammar> {
        // First, check if this looks like a lexer specification (.l file) instead of grammar
        self.validate_not_lexer()?;

        // Auto-detect textbook notation:
        //   S → if ( E ) S else S
        //   S -> if ( E ) S
        //   S : other
        // If the input uses → or -> arrows and has no %% separator or %token
        // declarations, parse it as simple textbook grammar notation.
        if Self::is_textbook_notation(self.input) {
            return self.parse_textbook();
        }

        // 0. Parse prologue %{ ... %}
        self.parse_prologue();

        // 1. Declarations section (before %%)
        self.parse_declarations()?;

        // 2. Separator
        self.skip_whitespace_and_comments();
        if self.consume("%%") {
            // 3. Rules section
            self.parse_rules()?;

            // 4. Optional epilogue section (after second %% in Bison format)
            self.skip_whitespace_and_comments();
            if self.consume("%%") {
                let rest = self.input[self.pos..].trim();
                if !rest.is_empty() {
                    self.grammar.epilogue = Some(rest.to_string());
                }
            }
        } else {
            return Err(Error::GrammarError {
                line: self.line,
                message: "Missing %% separator".to_string(),
            });
        }

        // 5. Default start symbol if not specified
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
                    self.grammar.token_types.insert(name.clone(), tag.clone());
                }

                // Parse optional literal value (e.g., %token PLUS "+" or %token IF 'if')
                self.skip_whitespace();
                if self.peek_char() == '"' || self.peek_char() == '\'' {
                    let quote = self.peek_char();
                    self.advance();
                    let start = self.pos;
                    while !self.is_eof() && self.peek_char() != quote {
                        if self.peek_char() == '\\' {
                            self.advance();
                            if !self.is_eof() {
                                self.advance();
                            }
                        } else {
                            self.advance();
                        }
                    }
                    let literal = self.input[start..self.pos].to_string();
                    self.advance(); // consume closing quote
                    self.grammar.token_literals.insert(name, literal);
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
                line: self.line,
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
                line: self.line,
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
                line: self.line,
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
                line: self.line,
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
                line: self.line,
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

            // Handle both identifiers and character literals
            if self.peek_char() == '\'' || self.peek_char() == '"' {
                // Parse character/string literal and convert to token name
                let (token_name, literal_value) = self.parse_char_literal()?;
                symbols.push(token_name.clone());

                // Record the literal value for later reference
                self.grammar.token_literals.insert(token_name, literal_value);
            } else {
                let name = self.parse_ident()?;
                if !name.is_empty() {
                    symbols.push(name);
                } else {
                    break;
                }
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
                    line: self.line,
                    message: format!("Expected ':' after rule '{}'", lhs),
                });
            }

            // Parse alternatives
            loop {
                // Use the new signature to capture rhs, action, and precedence
                let (rhs, action, prec) = self.parse_rhs()?;

                // First, add any pending mid-rule action rules (must come before the main rule)
                for mid_rule in self.pending_mid_rule_actions.drain(..) {
                    self.grammar.rules.push(mid_rule);
                }

                // Now add the main rule
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
        let mut final_action = None;
        let mut prec = None;
        let mut mid_rule_count = 0;

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
                let action = self.parse_action_block()?;

                // Check if this is a mid-rule action or final action
                self.skip_whitespace_and_comments();

                // Peek ahead to see if there are more symbols or %prec after this action
                let next_char = self.peek_char();
                let is_mid_rule = next_char != '|' && next_char != ';' && !self.is_eof()
                    && next_char != '\0' && !self.peek_str("%prec");

                // Also check if the next thing is another rule symbol
                if is_mid_rule {
                    // This is a mid-rule action - create a synthetic nonterminal
                    mid_rule_count += 1;
                    let synthetic_name = format!("@{}", self.grammar.rules.len() + mid_rule_count);

                    // Create the synthetic rule (will be added after main rule parsing)
                    self.pending_mid_rule_actions.push(Rule {
                        lhs: synthetic_name.clone(),
                        rhs: vec![], // Empty RHS (epsilon production)
                        action: Some(action),
                        precedence_sym: None,
                    });

                    // Add synthetic nonterminal to the current RHS
                    rhs.push(Symbol::NonTerminal(synthetic_name));
                } else {
                    // This is the final action
                    final_action = Some(action);
                    break;
                }
            } else if self.peek_char() == '|' || self.peek_char() == ';' || self.is_eof() {
                break;
            } else if self.peek_char() == '\'' || self.peek_char() == '"' {
                // Parse character/string literal: 'x', '\n', "keyword", etc.
                let (literal_token, literal_value) = self.parse_char_literal()?;

                // Auto-declare as token if not already declared
                if !self.grammar.tokens.contains(&literal_token) {
                    self.grammar.tokens.push(literal_token.clone());
                }

                // Record the literal value for lexer generation
                self.grammar.token_literals.insert(literal_token.clone(), literal_value);

                rhs.push(Symbol::Terminal(literal_token));
            } else {
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
        }
        Ok((rhs, final_action, prec))
    }

    fn parse_action_block(&mut self) -> Result<String> {
        // Consume { ... } coping with nested braces, but skip braces
        // inside string literals, char literals, and comments.
        if !self.consume("{") {
            return Ok(String::new());
        }

        let mut depth = 1;
        let start = self.pos;

        while depth > 0 && !self.is_eof() {
            let c = self.peek_char();

            // Skip string literals: "..."
            if c == '"' {
                self.advance();
                while !self.is_eof() && self.peek_char() != '"' {
                    if self.peek_char() == '\\' {
                        self.advance(); // skip escape
                    }
                    self.advance();
                }
                if !self.is_eof() {
                    self.advance(); // consume closing "
                }
                continue;
            }

            // Skip char literals: '...'
            if c == '\'' {
                self.advance();
                while !self.is_eof() && self.peek_char() != '\'' {
                    if self.peek_char() == '\\' {
                        self.advance(); // skip escape
                    }
                    self.advance();
                }
                if !self.is_eof() {
                    self.advance(); // consume closing '
                }
                continue;
            }

            // Skip line comments: // ...
            if c == '/' && self.peek_str("//") {
                while !self.is_eof() && self.peek_char() != '\n' {
                    self.advance();
                }
                continue;
            }

            // Skip block comments: /* ... */
            if c == '/' && self.peek_str("/*") {
                self.advance(); // /
                self.advance(); // *
                while !self.is_eof() && !self.peek_str("*/") {
                    self.advance();
                }
                if !self.is_eof() {
                    self.advance(); // *
                    self.advance(); // /
                }
                continue;
            }

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
            if c == '\n' {
                self.line += 1;
            }
            self.pos += c.len_utf8();
        }
    }

    fn consume(&mut self, s: &str) -> bool {
        if self.input[self.pos..].starts_with(s) {
            // Count newlines in consumed string
            self.line += s.chars().filter(|&c| c == '\n').count();
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

    /// Parse a character literal like 'x', '\n', or a string literal like "keyword"
    /// Returns: (token_name, literal_value)
    fn parse_char_literal(&mut self) -> Result<(String, String)> {
        let quote = self.peek_char();
        self.advance(); // consume opening quote

        let mut literal = String::new();
        while !self.is_eof() && self.peek_char() != quote {
            if self.peek_char() == '\\' && quote == '\'' {
                // Handle escape sequences like '\n', '\t'
                self.advance();
                if !self.is_eof() {
                    let escaped = self.peek_char();
                    literal.push('\\');
                    literal.push(escaped);
                    self.advance();
                }
            } else if self.peek_char() == '\\' && quote == '"' {
                // In double-quoted strings, keep backslash as-is for double escaping
                literal.push(self.peek_char());
                self.advance();
            } else {
                literal.push(self.peek_char());
                self.advance();
            }
        }

        if self.peek_char() == quote {
            self.advance(); // consume closing quote
        }

        // Generate a unique token name from the literal
        let token_name = match quote {
            '\'' => {
                // Single-quoted: map escapes like \n -> NEWLINE, \t -> TAB, or just use the character
                match literal.as_str() {
                    "\\n" => "NEWLINE".to_string(),
                    "\\t" => "TAB".to_string(),
                    "\\r" => "RETURN".to_string(),
                    _ if literal.len() == 1 => {
                        // Single character: +_PLUS, *_STAR, etc.
                        let ch = literal.chars().next().unwrap();
                        match ch {
                            '+' => "PLUS".to_string(),
                            '-' => "MINUS".to_string(),
                            '*' => "STAR".to_string(),
                            '/' => "SLASH".to_string(),
                            '%' => "PERCENT".to_string(),
                            '=' => "ASSIGN".to_string(),
                            '<' => "LT".to_string(),
                            '>' => "GT".to_string(),
                            '!' => "NOT".to_string(),
                            '&' => "AMP".to_string(),
                            '|' => "PIPE".to_string(),
                            '(' => "LPAREN".to_string(),
                            ')' => "RPAREN".to_string(),
                            '[' => "LBRACKET".to_string(),
                            ']' => "RBRACKET".to_string(),
                            '{' => "LBRACE".to_string(),
                            '}' => "RBRACE".to_string(),
                            ';' => "SEMICOLON".to_string(),
                            ',' => "COMMA".to_string(),
                            '.' => "DOT".to_string(),
                            ':' => "COLON".to_string(),
                            '?' => "QUESTION".to_string(),
                            '^' => "CARET".to_string(),
                            '~' => "TILDE".to_string(),
                            '@' => "AT".to_string(),
                            '#' => "HASH".to_string(),
                            '$' => "DOLLAR".to_string(),
                            '\\' => "BACKSLASH".to_string(),
                            _ => format!("CHAR_{:x}", ch as u32),
                        }
                    }
                    _ => format!("CHAR_{}", literal.chars().next().map(|c| c as u32).unwrap_or(0)),
                }
            }
            '"' => {
                // Double-quoted keyword: uppercase it
                literal.to_uppercase()
            }
            _ => "UNKNOWN".to_string(),
        };

        Ok((token_name, literal))
    }


    // =========================================================================
    // Textbook notation support
    // =========================================================================
    //
    // Bison classifies symbols like this (from reader.c & symtab.h):
    //   - Symbols declared with %token → terminal (token_sym)
    //   - Symbols appearing as LHS of a rule → nonterminal (nterm_sym)
    //   - Remaining unknown symbols → error
    //
    // For textbook notation we do the same thing automatically:
    //   1. Collect all LHS symbols → these are non-terminals
    //   2. Everything else referenced in RHS → terminal (auto-declared)
    //   3. First rule's LHS → start symbol
    //   4. Single-char punctuation in the grammar → literal terminals

    /// Detect whether the input uses textbook grammar notation rather than
    /// Bison-style .y format. Returns true if:
    ///   - Input contains → or -> as a rule separator
    ///   - Input does NOT contain %% (Bison section separator)
    ///   - Input does NOT contain %token declarations
    fn is_textbook_notation(input: &str) -> bool {
        let has_bison_markers = input.contains("%%") || input.contains("%token");
        if has_bison_markers {
            return false;
        }

        // Look for arrow notation - support many Unicode arrow variants
        // that users might paste from Word, PDFs, or textbooks
        for line in input.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed.starts_with("//") || trimmed.starts_with("#") {
                continue;
            }
            if trimmed.contains("->") || trimmed.contains("::=") || Self::contains_arrow(trimmed) {
                return true;
            }
        }

        false
    }

    /// Check if a line contains any Unicode arrow character commonly used in grammars.
    fn contains_arrow(s: &str) -> bool {
        for ch in s.chars() {
            if Self::is_arrow_char(ch) {
                return true;
            }
        }
        false
    }

    /// Returns true if the character is a Unicode arrow commonly used in grammar notation.
    fn is_arrow_char(ch: char) -> bool {
        matches!(
            ch,
            '\u{2192}'   // → RIGHTWARDS ARROW (most common)
            | '\u{21D2}' // ⇒ RIGHTWARDS DOUBLE ARROW
            | '\u{27F6}' // ⟶ LONG RIGHTWARDS ARROW
            | '\u{2794}' // ➔ HEAVY WIDE-HEADED RIGHTWARDS ARROW
            | '\u{279C}' // ➜ HEAVY ROUND-TIPPED RIGHTWARDS ARROW
            | '\u{279D}' // ➝ TRIANGLE-HEADED RIGHTWARDS ARROW
            | '\u{279E}' // ➞ HEAVY TRIANGLE-HEADED RIGHTWARDS ARROW
            | '\u{21A6}' // ↦ RIGHTWARDS ARROW FROM BAR
            | '\u{2B62}' // ⭢ RIGHTWARDS TRIANGLE-HEADED ARROW
            | '\u{2B95}' // ⮕ RIGHTWARDS BLACK ARROW
            | '\u{21FE}' // ⇾ RIGHTWARDS OPEN-HEADED ARROW
            | '\u{21E2}' // ⇢ RIGHTWARDS DASHED ARROW
            | '\u{21E8}' // ⇨ RIGHTWARDS WHITE ARROW
            | '\u{27A1}' // ➡ BLACK RIGHTWARDS ARROW
        )
    }

    /// Find the first Unicode arrow in a string, returning its byte position and UTF-8 length.
    fn find_arrow(s: &str) -> Option<(usize, usize)> {
        for (byte_idx, ch) in s.char_indices() {
            if Self::is_arrow_char(ch) {
                return Some((byte_idx, ch.len_utf8()));
            }
        }
        None
    }

    /// Parse textbook notation grammar format:
    ///
    /// ```text
    /// S → if ( E ) S else S
    /// S → if ( E ) S
    /// S → other
    /// E → condition
    /// ```
    ///
    /// Also supports: ->, ::=, and | for alternatives:
    /// ```text
    /// S -> if ( E ) S else S | if ( E ) S | other
    /// E -> condition
    /// ```
    fn parse_textbook(&mut self) -> Result<Grammar> {
        use std::collections::HashSet;

        let input = self.input.to_string();
        let mut rules: Vec<(String, Vec<String>)> = Vec::new();
        let mut lhs_set: HashSet<String> = HashSet::new();
        let mut all_rhs_symbols: Vec<String> = Vec::new();
        let mut literal_map: HashMap<String, String> = HashMap::new();

        // First pass: collect all rules and identify LHS symbols
        let mut current_lhs: Option<String> = None;

        for (line_num, line) in input.lines().enumerate() {
            let trimmed = line.trim();

            // Skip empty lines and comments
            if trimmed.is_empty()
                || trimmed.starts_with("//")
                || trimmed.starts_with("#")
                || trimmed.starts_with("/*")
            {
                continue;
            }

            // Try to split on arrow: any Unicode arrow, ->, or ::=
            let (lhs_part, rhs_part) = if let Some((idx, arrow_len)) = Self::find_arrow(trimmed) {
                let lhs = trimmed[..idx].trim();
                let rhs = trimmed[idx + arrow_len..].trim();
                (Some(lhs.to_string()), rhs.to_string())
            } else if let Some(idx) = trimmed.find("::=") {
                let lhs = trimmed[..idx].trim();
                let rhs = trimmed[idx + 3..].trim();
                (Some(lhs.to_string()), rhs.to_string())
            } else if let Some(idx) = trimmed.find("->") {
                let lhs = trimmed[..idx].trim();
                let rhs = trimmed[idx + 2..].trim();
                (Some(lhs.to_string()), rhs.to_string())
            } else if trimmed.starts_with('|') {
                // Continuation alternative for the previous LHS
                let rhs = trimmed[1..].trim();
                (None, rhs.to_string())
            } else {
                // Try colon notation: LHS : RHS (but only if no Bison %% markers)
                if let Some(idx) = trimmed.find(':') {
                    let lhs = trimmed[..idx].trim();
                    let rhs = trimmed[idx + 1..].trim();
                    // Only treat as a rule if LHS looks like a single identifier
                    if !lhs.is_empty()
                        && lhs.chars().all(|c| c.is_alphanumeric() || c == '_' || c == '\'')
                    {
                        (Some(lhs.to_string()), rhs.to_string())
                    } else {
                        continue;
                    }
                } else {
                    continue;
                }
            };

            // Update current LHS if we got one (uppercase for consistency)
            if let Some(ref lhs) = lhs_part {
                if lhs.is_empty() {
                    return Err(Error::GrammarError {
                        line: line_num + 1,
                        message: "Empty left-hand side in rule".to_string(),
                    });
                }
                let lhs_upper = lhs.to_uppercase();
                current_lhs = Some(lhs_upper.clone());
                lhs_set.insert(lhs_upper);
            }

            let active_lhs = match &current_lhs {
                Some(l) => l.clone(),
                None => {
                    return Err(Error::GrammarError {
                        line: line_num + 1,
                        message: "Rule alternative '|' without a preceding rule".to_string(),
                    });
                }
            };

            // Split RHS on | for alternatives
            let alternatives: Vec<&str> = rhs_part.split('|').collect();
            for alt in alternatives {
                let alt = alt.trim();
                if alt.is_empty() {
                    // Empty alternative = epsilon production
                    rules.push((active_lhs.clone(), Vec::new()));
                    continue;
                }

                // Tokenize the RHS: split on whitespace, but handle quoted strings
                let raw_symbols = Self::tokenize_textbook_rhs(alt);
                let symbols: Vec<String> = raw_symbols
                    .iter()
                    .map(|s| {
                        // Normalize single-char punctuation to token names
                        // so grammar and lexer agree on terminal names
                        if s.len() == 1 && !s.chars().next().unwrap().is_alphanumeric() {
                            let name = token_name_for(s);
                            // Record original literal for lexer generation
                            literal_map.insert(name.clone(), s.clone());
                            name
                        } else {
                            // Lowercase keywords: record the original for lexer matching
                            let upper = s.to_uppercase();
                            if upper != *s {
                                literal_map.insert(upper.clone(), s.clone());
                            }
                            upper
                        }
                    })
                    .collect();
                for sym in &symbols {
                    all_rhs_symbols.push(sym.clone());
                }
                rules.push((active_lhs.clone(), symbols));
            }
        }

        if rules.is_empty() {
            return Err(Error::GrammarError {
                line: 0,
                message: "No grammar rules found. Use notation like: S → a B c".to_string(),
            });
        }

        // Second pass: classify symbols (Bison-style, as in symtab.c)
        // Symbols appearing as LHS → nonterminal
        // Everything else → terminal (auto-declared)
        let mut tokens: Vec<String> = Vec::new();
        let mut token_set: HashSet<String> = HashSet::new();

        for sym in &all_rhs_symbols {
            if !lhs_set.contains(sym) && token_set.insert(sym.clone()) {
                tokens.push(sym.clone());
            }
        }

        // Build the Grammar
        self.grammar.tokens = tokens;
        self.grammar.token_literals = literal_map;
        self.grammar.start_symbol = rules[0].0.clone();

        for (lhs, rhs_syms) in &rules {
            let rhs: Vec<Symbol> = rhs_syms
                .iter()
                .map(|s| {
                    if lhs_set.contains(s) {
                        Symbol::NonTerminal(s.clone())
                    } else {
                        Symbol::Terminal(s.clone())
                    }
                })
                .collect();

            self.grammar.rules.push(Rule {
                lhs: lhs.clone(),
                rhs,
                action: None,
                precedence_sym: None,
            });
        }

        Ok(self.grammar.clone())
    }

    /// Tokenize a textbook RHS string into individual symbols.
    /// Handles:
    ///   - Whitespace-separated identifiers: `if ( E ) S else S`
    ///   - Quoted literals: `'if'`, `"+"`, `'('`
    ///   - Single punctuation characters treated as individual tokens
    fn tokenize_textbook_rhs(rhs: &str) -> Vec<String> {
        let mut symbols = Vec::new();
        let mut chars = rhs.chars().peekable();

        while let Some(&ch) = chars.peek() {
            // Skip whitespace
            if ch.is_whitespace() {
                chars.next();
                continue;
            }

            // Quoted literal: 'x' or "x"
            if ch == '\'' || ch == '"' {
                let quote = ch;
                chars.next(); // consume opening quote
                let mut lit = String::new();
                while let Some(&c) = chars.peek() {
                    if c == quote {
                        chars.next(); // consume closing quote
                        break;
                    }
                    lit.push(c);
                    chars.next();
                }
                if !lit.is_empty() {
                    symbols.push(lit);
                }
                continue;
            }

            // Identifier: alphanumeric or underscore (also allow ' for primed symbols like E')
            if ch.is_alphanumeric() || ch == '_' {
                let mut ident = String::new();
                while let Some(&c) = chars.peek() {
                    if c.is_alphanumeric() || c == '_' || c == '\'' {
                        ident.push(c);
                        chars.next();
                    } else {
                        break;
                    }
                }
                symbols.push(ident);
                continue;
            }

            // ε or epsilon → skip (empty production marker)
            if ch == '\u{03B5}' {
                chars.next();
                continue;
            }

            // Single punctuation character → treat as a terminal symbol
            // Map common punctuation to token names for compatibility
            chars.next();
            symbols.push(ch.to_string());
        }

        symbols
    }

    // =========================================================================
    // Lexer misdetection guard
    // =========================================================================

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

/// Convert a token string to an uppercase token name suitable for use in
/// generated lexer/parser code. Single-character punctuation gets a
/// descriptive name (like Bison does), identifiers get uppercased.
fn token_name_for(token: &str) -> String {
    if token.len() == 1 {
        match token.chars().next().unwrap() {
            '(' => "LPAREN".to_string(),
            ')' => "RPAREN".to_string(),
            '[' => "LBRACKET".to_string(),
            ']' => "RBRACKET".to_string(),
            '{' => "LBRACE".to_string(),
            '}' => "RBRACE".to_string(),
            '+' => "PLUS".to_string(),
            '-' => "MINUS".to_string(),
            '*' => "TIMES".to_string(),
            '/' => "DIVIDE".to_string(),
            '=' => "EQUALS".to_string(),
            '<' => "LT".to_string(),
            '>' => "GT".to_string(),
            ',' => "COMMA".to_string(),
            ';' => "SEMICOLON".to_string(),
            '.' => "DOT".to_string(),
            ':' => "COLON".to_string(),
            '!' => "BANG".to_string(),
            '&' => "AMP".to_string(),
            '^' => "CARET".to_string(),
            '%' => "PERCENT".to_string(),
            '~' => "TILDE".to_string(),
            '#' => "HASH".to_string(),
            '@' => "AT".to_string(),
            '?' => "QUESTION".to_string(),
            c => format!("CHAR_{}", c as u32),
        }
    } else {
        token.to_uppercase()
    }
}
