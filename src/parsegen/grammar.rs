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
// use std::collections::{HashMap, HashSet};

/// Abstract Syntax Tree for Grammar.
#[derive(Debug, Clone)]
pub struct Grammar {
    pub tokens: Vec<String>,
    pub start_symbol: String,
    pub rules: Vec<Rule>,
    pub precedence: Vec<Precedence>,
}

#[derive(Debug, Clone)]
pub struct Rule {
    pub lhs: String,
    pub rhs: Vec<Symbol>,
    pub action: Option<String>, // Logic code { ... }
    pub precedence_sym: Option<String>, // %prec
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
        }
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
        
        // 0. Skip prologue %{ ... %}
        self.skip_prologue();
        
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
    
    fn skip_prologue(&mut self) {
        self.skip_whitespace_and_comments();
        if self.consume("%{") {
            // Skip until %}
            while !self.is_eof() {
                if self.consume("%}") {
                    break;
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
            } else {
                // Ignore unknown decls or comments
                self.advance(); 
            }
        }
        Ok(())
    }

    fn parse_token_decl(&mut self) -> Result<()> {
        loop {
            self.skip_whitespace();
            if self.peek_char() == '%' || self.is_eof() {
                break;
            }
            let name = self.parse_ident()?;
            if !name.is_empty() {
                self.grammar.tokens.push(name);
            } else {
                break;
            }
        }
        Ok(())
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
        self.grammar.precedence.push(Precedence { assoc, symbols: symbols.clone() });
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
            if lhs.is_empty() { break; } // EOF or error

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
            if name.is_empty() { break; } 
            
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
        if !self.consume("{") { return Ok(String::new()); }
        
        let mut depth = 1;
        let start = self.pos;
        
        while depth > 0 && !self.is_eof() {
            let c = self.peek_char();
            if c == '{' { depth += 1; }
            if c == '}' { depth -= 1; }
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
            if (trimmed.contains("[a-z") || trimmed.contains("[A-Z") || 
                trimmed.contains("[0-9") || trimmed.contains("\\s") ||
                trimmed.contains("\\d") || trimmed.contains("\\w")) &&
               !trimmed.starts_with("%") {
                lexer_indicators += 2;
                if indicator_examples.len() < 2 {
                    indicator_examples.push(format!("'{}' (regex pattern)", trimmed.chars().take(50).collect::<String>()));
                }
            }
            
            // Check for return TOKEN patterns: { return SOMETHING; }
            if trimmed.contains("return ") && trimmed.contains(";") && 
               (trimmed.contains("{") || line.ends_with("}")) {
                lexer_indicators += 2;
                if indicator_examples.len() < 2 {
                    indicator_examples.push(format!("'{}' (lexer action)", trimmed.chars().take(50).collect::<String>()));
                }
            }
            
            // Check for quoted literal patterns followed by action
            if (trimmed.starts_with('"') || trimmed.starts_with("'")) && 
               trimmed.contains('{') {
                lexer_indicators += 1;
                if indicator_examples.len() < 2 {
                    indicator_examples.push(format!("'{}' (literal pattern with action)", trimmed.chars().take(40).collect::<String>()));
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
                    indicator_examples.push(format!("'{}' (skip action)", trimmed.chars().take(40).collect::<String>()));
                }
            }
            
            // Check for regex quantifiers like + * ? at end of pattern
            if (trimmed.contains("]+") || trimmed.contains(")*") || 
                trimmed.contains(")?") || trimmed.contains(".+") || 
                trimmed.contains(".*")) && 
               !trimmed.starts_with("%") && !trimmed.starts_with("|") {
                lexer_indicators += 1;
            }
        }
        
        // If we found strong evidence of lexer syntax, return an error
        if lexer_indicators >= 5 {
            let examples_str = if indicator_examples.is_empty() {
                String::new()
            } else {
                format!("\n\nDetected lexer patterns:\n  - {}", indicator_examples.join("\n  - "))
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
