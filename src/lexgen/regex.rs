//! Regex AST and Parser.
//!
//! Parses simple regular expressions into an Abstract Syntax Tree (AST).
//! Supported syntax:
//! - Literals: `a`, `b`, `1`
//! - Concatenation: `ab`
//! - Alternation: `a|b`
//! - Kleene Star: `a*`
//! - Plus: `a+`
//! - Question: `a?`
//! - Grouping: `(a|b)`
//! - Character classes: `[a-z]`, `[^0-9]`, `[abc]`
//! - Dot (any char): `.`
//! - Escape sequences: `\d`, `\w`, `\s`, `\D`, `\W`, `\S`, `\n`, `\t`, etc.

use crate::error::{Error, Result};
use std::iter::Peekable;
use std::str::Chars;

/// Abstract Syntax Tree for Regular Expressions.
#[derive(Debug, Clone, PartialEq)]
pub enum Regex {
    /// Matches an empty string (epsilon).
    Empty,
    /// Matches a specific character.
    Literal(char),
    /// Matches any single character in the set.
    CharClass(CharClass),
    /// Matches any character except newline.
    Dot,
    /// Matches valid concatenation of two expressions.
    Concat(Box<Regex>, Box<Regex>),
    /// Matches either LHS or RHS.
    Union(Box<Regex>, Box<Regex>),
    /// Matches zero or more occurrences.
    Star(Box<Regex>),
    /// Matches one or more occurrences.
    Plus(Box<Regex>),
    /// Matches zero or one occurrence.
    Question(Box<Regex>),
}

/// Represents a character class like [a-z] or [^0-9].
#[derive(Debug, Clone, PartialEq)]
pub struct CharClass {
    /// If true, this is a negated class [^...].
    pub negated: bool,
    /// The ranges and individual characters in the class.
    pub ranges: Vec<CharRange>,
}

/// A single character or a range of characters.
#[derive(Debug, Clone, PartialEq)]
pub enum CharRange {
    /// A single character.
    Single(char),
    /// A range like a-z (inclusive).
    Range(char, char),
}

impl CharClass {
    /// Returns true if the character matches this class.
    pub fn matches(&self, c: char) -> bool {
        let in_class = self.ranges.iter().any(|range| match range {
            CharRange::Single(ch) => *ch == c,
            CharRange::Range(start, end) => c >= *start && c <= *end,
        });
        if self.negated { !in_class } else { in_class }
    }

    /// Expands the character class into a list of all matching characters.
    /// For ASCII range only (0-127). Used for NFA construction.
    pub fn expand(&self) -> Vec<char> {
        let mut chars = Vec::new();
        for code in 0u8..128u8 {
            let c = code as char;
            if self.matches(c) {
                chars.push(c);
            }
        }
        chars
    }
}

/// Wrapper for the AST.
#[derive(Debug, Clone)]
pub struct RegexAst {
    pub root: Regex,
}

impl RegexAst {
    /// Parses a regex string into an AST.
    pub fn parse(pattern: &str) -> Result<Self> {
        let mut parser = RegexParser::new(pattern);
        let root = parser.parse_union()?;
        
        // Ensure entire pattern is consumed
        if parser.chars.peek().is_some() {
             return Err(Error::RegexError {
                position: parser.position(),
                message: "Unexpected character at end of pattern".to_string(),
            });
        }

        Ok(RegexAst { root })
    }
}

/// Recursive descent parser for regex.
struct RegexParser<'a> {
    chars: Peekable<Chars<'a>>,
    current_index: usize,
}

impl<'a> RegexParser<'a> {
    fn new(input: &'a str) -> Self {
        Self {
            chars: input.chars().peekable(),
            current_index: 0,
        }
    }

    fn position(&self) -> usize {
        self.current_index
    }

    fn peek(&mut self) -> Option<char> {
        self.chars.peek().copied()
    }

    fn advance(&mut self) -> Option<char> {
        let c = self.chars.next();
        if c.is_some() {
            self.current_index += 1;
        }
        c
    }

    fn consume(&mut self, expected: char) -> bool {
        if let Some(c) = self.peek() {
            if c == expected {
                self.advance();
                return true;
            }
        }
        false
    }
    
    // Precedence: Star/Plus/Question > Concat > Union

    /// Parses alternation: Term | Term | ...
    fn parse_union(&mut self) -> Result<Regex> {
        let mut lhs = self.parse_concat()?;

        while self.consume('|') {
            let rhs = self.parse_concat()?;
            lhs = Regex::Union(Box::new(lhs), Box::new(rhs));
        }

        Ok(lhs)
    }

    /// Parses concatenation: Factor Factor ...
    fn parse_concat(&mut self) -> Result<Regex> {
        let mut lhs = self.parse_factor()?;

        // While we have characters that can start a factor, keep parsing and concatenating
        while let Some(c) = self.peek() {
            if c == '|' || c == ')' {
                break;
            }
            let rhs = self.parse_factor()?;
            lhs = Regex::Concat(Box::new(lhs), Box::new(rhs));
        }

        Ok(lhs)
    }

    /// Parses factors and repetition suffixes: Atom*, Atom+, Atom?
    fn parse_factor(&mut self) -> Result<Regex> {
        let mut atom = self.parse_atom()?;

        while let Some(c) = self.peek() {
            match c {
                '*' => {
                    self.advance();
                    atom = Regex::Star(Box::new(atom));
                }
                '+' => {
                    self.advance();
                    atom = Regex::Plus(Box::new(atom));
                }
                '?' => {
                    self.advance();
                    atom = Regex::Question(Box::new(atom));
                }
                _ => break,
            }
        }

        Ok(atom)
    }

    /// Parses atoms: (Expr), [CharClass], ., Escaped chars, Literals
    fn parse_atom(&mut self) -> Result<Regex> {
        match self.peek() {
            Some('(') => {
                self.advance();
                let inner = self.parse_union()?;
                if !self.consume(')') {
                    return Err(Error::RegexError {
                        position: self.position(),
                        message: "Missing closing parenthesis".to_string(),
                    });
                }
                Ok(inner)
            }
            Some('[') => {
                self.parse_char_class()
            }
            Some('.') => {
                self.advance();
                Ok(Regex::Dot)
            }
            Some('\\') => {
                self.advance();
                self.parse_escape()
            }
            Some(c) if is_special(c) => {
                 Err(Error::RegexError {
                    position: self.position(),
                    message: format!("Unexpected special character '{}'", c),
                })
            }
            Some(c) => {
                self.advance();
                Ok(Regex::Literal(c))
            }
            None => Ok(Regex::Empty),
        }
    }

    /// Parses an escape sequence after the backslash has been consumed.
    fn parse_escape(&mut self) -> Result<Regex> {
        match self.advance() {
            Some('d') => {
                // \d = [0-9]
                Ok(Regex::CharClass(CharClass {
                    negated: false,
                    ranges: vec![CharRange::Range('0', '9')],
                }))
            }
            Some('D') => {
                // \D = [^0-9]
                Ok(Regex::CharClass(CharClass {
                    negated: true,
                    ranges: vec![CharRange::Range('0', '9')],
                }))
            }
            Some('w') => {
                // \w = [a-zA-Z0-9_]
                Ok(Regex::CharClass(CharClass {
                    negated: false,
                    ranges: vec![
                        CharRange::Range('a', 'z'),
                        CharRange::Range('A', 'Z'),
                        CharRange::Range('0', '9'),
                        CharRange::Single('_'),
                    ],
                }))
            }
            Some('W') => {
                // \W = [^a-zA-Z0-9_]
                Ok(Regex::CharClass(CharClass {
                    negated: true,
                    ranges: vec![
                        CharRange::Range('a', 'z'),
                        CharRange::Range('A', 'Z'),
                        CharRange::Range('0', '9'),
                        CharRange::Single('_'),
                    ],
                }))
            }
            Some('s') => {
                // \s = [ \t\n\r\f\v]
                Ok(Regex::CharClass(CharClass {
                    negated: false,
                    ranges: vec![
                        CharRange::Single(' '),
                        CharRange::Single('\t'),
                        CharRange::Single('\n'),
                        CharRange::Single('\r'),
                        CharRange::Single('\x0C'), // form feed
                        CharRange::Single('\x0B'), // vertical tab
                    ],
                }))
            }
            Some('S') => {
                // \S = [^ \t\n\r\f\v]
                Ok(Regex::CharClass(CharClass {
                    negated: true,
                    ranges: vec![
                        CharRange::Single(' '),
                        CharRange::Single('\t'),
                        CharRange::Single('\n'),
                        CharRange::Single('\r'),
                        CharRange::Single('\x0C'),
                        CharRange::Single('\x0B'),
                    ],
                }))
            }
            Some('n') => Ok(Regex::Literal('\n')),
            Some('t') => Ok(Regex::Literal('\t')),
            Some('r') => Ok(Regex::Literal('\r')),
            Some('f') => Ok(Regex::Literal('\x0C')),
            Some('v') => Ok(Regex::Literal('\x0B')),
            Some('0') => Ok(Regex::Literal('\0')),
            Some(c) => {
                // Any other character is escaped literally (e.g., \*, \[, \\)
                Ok(Regex::Literal(c))
            }
            None => Err(Error::RegexError {
                position: self.position(),
                message: "Unexpected end of pattern after backslash".to_string(),
            }),
        }
    }

    /// Parses a character class [...]
    fn parse_char_class(&mut self) -> Result<Regex> {
        // Consume '['
        self.advance();
        
        let negated = if self.peek() == Some('^') {
            self.advance();
            true
        } else {
            false
        };

        let mut ranges = Vec::new();
        
        // Handle ] as first character (it's literal in that position)
        if self.peek() == Some(']') {
            self.advance();
            ranges.push(CharRange::Single(']'));
        }

        while let Some(c) = self.peek() {
            if c == ']' {
                self.advance();
                return Ok(Regex::CharClass(CharClass { negated, ranges }));
            }

            let first = self.parse_char_class_char()?;
            
            // Check for range
            if self.peek() == Some('-') {
                // Peek ahead to see if this is a range or literal dash
                self.advance(); // consume '-'
                
                if self.peek() == Some(']') {
                    // Dash at end is literal
                    ranges.push(CharRange::Single(first));
                    ranges.push(CharRange::Single('-'));
                } else if let Some(_) = self.peek() {
                    let second = self.parse_char_class_char()?;
                    if first > second {
                        return Err(Error::RegexError {
                            position: self.position(),
                            message: format!("Invalid character range: '{}'-'{}'", first, second),
                        });
                    }
                    ranges.push(CharRange::Range(first, second));
                } else {
                    // End of input after dash
                    return Err(Error::RegexError {
                        position: self.position(),
                        message: "Unexpected end of pattern in character class".to_string(),
                    });
                }
            } else {
                ranges.push(CharRange::Single(first));
            }
        }

        Err(Error::RegexError {
            position: self.position(),
            message: "Unclosed character class".to_string(),
        })
    }

    /// Parses a single character inside a character class (handles escapes).
    fn parse_char_class_char(&mut self) -> Result<char> {
        match self.peek() {
            Some('\\') => {
                self.advance();
                match self.advance() {
                    Some('n') => Ok('\n'),
                    Some('t') => Ok('\t'),
                    Some('r') => Ok('\r'),
                    Some('f') => Ok('\x0C'),
                    Some('v') => Ok('\x0B'),
                    Some('0') => Ok('\0'),
                    Some(c) => Ok(c), // Escaped literal
                    None => Err(Error::RegexError {
                        position: self.position(),
                        message: "Unexpected end after backslash in character class".to_string(),
                    }),
                }
            }
            Some(c) => {
                self.advance();
                Ok(c)
            }
            None => Err(Error::RegexError {
                position: self.position(),
                message: "Unexpected end of character class".to_string(),
            }),
        }
    }
}

fn is_special(c: char) -> bool {
    matches!(c, '*' | '+' | '?' | '|' | ')' )
    // '(' and '[' and '.' handled explicitly
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_literal() {
        let ast = RegexAst::parse("a").unwrap();
        assert_eq!(ast.root, Regex::Literal('a'));
    }

    #[test]
    fn test_concat() {
        let ast = RegexAst::parse("ab").unwrap();
        assert_eq!(ast.root, Regex::Concat(
            Box::new(Regex::Literal('a')),
            Box::new(Regex::Literal('b'))
        ));
    }

    #[test]
    fn test_union() {
        let ast = RegexAst::parse("a|b").unwrap();
        assert_eq!(ast.root, Regex::Union(
            Box::new(Regex::Literal('a')),
            Box::new(Regex::Literal('b'))
        ));
    }
    
    #[test]
    fn test_precedence() {
        // ab|c should be (ab)|c
        let _ = RegexAst::parse("ab|c").unwrap();
    }
}
