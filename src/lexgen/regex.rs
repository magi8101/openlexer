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
//! - Unicode escapes: `\u{XXXX}`, `\x{XXXX}`
//! - Unicode properties: `\p{Lu}`, `\p{Script=Greek}`, `\P{Nd}`

use crate::error::{Error, Result};
use crate::lexgen::unicode;
use std::iter::Peekable;
use std::ops::RangeInclusive;
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
    /// Unicode code point ranges (for properties like \p{Lu}).
    pub unicode_ranges: Vec<RangeInclusive<u32>>,
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
        }) || unicode::char_in_ranges(c, &self.unicode_ranges);

        if self.negated {
            !in_class
        } else {
            in_class
        }
    }

    /// Expands the character class into a list of all matching characters.
    /// For ASCII range only (0-127) by default. Use expand_unicode for full Unicode.
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

    /// Expands the character class for Unicode characters up to max_codepoint.
    /// For efficiency, only expands up to a reasonable limit.
    pub fn expand_unicode(&self, max_codepoint: u32) -> Vec<char> {
        let mut chars = Vec::new();
        let limit = std::cmp::min(max_codepoint, 0xFFFF); // Limit to BMP for performance

        for cp in 0..=limit {
            if let Some(c) = char::from_u32(cp) {
                if self.matches(c) {
                    chars.push(c);
                }
            }
        }
        chars
    }

    /// Returns true if this class contains any Unicode properties
    /// (requiring Unicode-aware expansion).
    pub fn has_unicode_properties(&self) -> bool {
        !self.unicode_ranges.is_empty()
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
            Some('[') => self.parse_char_class(),
            Some('.') => {
                self.advance();
                Ok(Regex::Dot)
            }
            Some('\\') => {
                self.advance();
                self.parse_escape()
            }
            Some(c) if is_special(c) => Err(Error::RegexError {
                position: self.position(),
                message: format!("Unexpected special character '{}'", c),
            }),
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
                    unicode_ranges: vec![],
                }))
            }
            Some('D') => {
                // \D = [^0-9]
                Ok(Regex::CharClass(CharClass {
                    negated: true,
                    ranges: vec![CharRange::Range('0', '9')],
                    unicode_ranges: vec![],
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
                    unicode_ranges: vec![],
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
                    unicode_ranges: vec![],
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
                    unicode_ranges: vec![],
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
                    unicode_ranges: vec![],
                }))
            }
            Some('u') | Some('x') => {
                // Unicode hex escape: \u{XXXX} or \uXXXX or \x{XXXX}
                self.parse_unicode_escape()
            }
            Some('p') => {
                // Unicode property: \p{Property}
                self.parse_unicode_property(false)
            }
            Some('P') => {
                // Negated Unicode property: \P{Property}
                self.parse_unicode_property(true)
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

    /// Parses a Unicode hex escape sequence: \u{XXXX} or \uXXXX or \x{XXXX}
    fn parse_unicode_escape(&mut self) -> Result<Regex> {
        if self.peek() == Some('{') {
            // Variable-length format: \u{XXXX} or \x{XXXX}
            self.advance(); // consume '{'

            let mut hex_str = String::new();
            while let Some(c) = self.peek() {
                if c == '}' {
                    self.advance();
                    break;
                }
                if c.is_ascii_hexdigit() {
                    self.advance();
                    hex_str.push(c);
                } else if c == ' ' {
                    // Spaces are allowed in \u{XX XX XX} format
                    self.advance();
                } else {
                    return Err(Error::RegexError {
                        position: self.position(),
                        message: format!("Invalid character '{}' in Unicode escape", c),
                    });
                }
            }

            if hex_str.is_empty() {
                return Err(Error::RegexError {
                    position: self.position(),
                    message: "Empty Unicode escape sequence".to_string(),
                });
            }

            let code_point = u32::from_str_radix(&hex_str, 16).map_err(|_| Error::RegexError {
                position: self.position(),
                message: format!("Invalid hex value in Unicode escape: {}", hex_str),
            })?;

            let c = char::from_u32(code_point).ok_or_else(|| Error::RegexError {
                position: self.position(),
                message: format!("Invalid Unicode code point: U+{:04X}", code_point),
            })?;

            Ok(Regex::Literal(c))
        } else {
            // Fixed 4-digit format: \uXXXX
            let mut hex_str = String::new();
            for _ in 0..4 {
                if let Some(c) = self.peek() {
                    if c.is_ascii_hexdigit() {
                        self.advance();
                        hex_str.push(c);
                    } else {
                        break;
                    }
                } else {
                    break;
                }
            }

            if hex_str.len() < 4 {
                return Err(Error::RegexError {
                    position: self.position(),
                    message: format!(
                        "Expected 4 hex digits in Unicode escape, got {}",
                        hex_str.len()
                    ),
                });
            }

            let code_point = u32::from_str_radix(&hex_str, 16).map_err(|_| Error::RegexError {
                position: self.position(),
                message: format!("Invalid hex value: {}", hex_str),
            })?;

            let c = char::from_u32(code_point).ok_or_else(|| Error::RegexError {
                position: self.position(),
                message: format!("Invalid Unicode code point: U+{:04X}", code_point),
            })?;

            Ok(Regex::Literal(c))
        }
    }

    /// Parses a Unicode property: \p{Property} or \P{Property}
    fn parse_unicode_property(&mut self, negated: bool) -> Result<Regex> {
        if self.peek() != Some('{') {
            return Err(Error::RegexError {
                position: self.position(),
                message: "Expected '{' after \\p or \\P".to_string(),
            });
        }
        self.advance(); // consume '{'

        let mut prop_str = String::new();
        let mut found_close = false;

        while let Some(c) = self.peek() {
            if c == '}' {
                self.advance();
                found_close = true;
                break;
            }
            self.advance();
            prop_str.push(c);
        }

        if !found_close {
            return Err(Error::RegexError {
                position: self.position(),
                message: "Unclosed Unicode property".to_string(),
            });
        }

        if prop_str.is_empty() {
            return Err(Error::RegexError {
                position: self.position(),
                message: "Empty Unicode property".to_string(),
            });
        }

        // Parse the property using unicode module
        let (ranges, _) = unicode::parse_property(&prop_str).ok_or_else(|| Error::RegexError {
            position: self.position(),
            message: format!("Unknown Unicode property: {}", prop_str),
        })?;

        Ok(Regex::CharClass(CharClass {
            negated,
            ranges: vec![],
            unicode_ranges: ranges,
        }))
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
                return Ok(Regex::CharClass(CharClass {
                    negated,
                    ranges,
                    unicode_ranges: vec![],
                }));
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
    matches!(c, '*' | '+' | '?' | '|' | ')')
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
        assert_eq!(
            ast.root,
            Regex::Concat(Box::new(Regex::Literal('a')), Box::new(Regex::Literal('b')))
        );
    }

    #[test]
    fn test_union() {
        let ast = RegexAst::parse("a|b").unwrap();
        assert_eq!(
            ast.root,
            Regex::Union(Box::new(Regex::Literal('a')), Box::new(Regex::Literal('b')))
        );
    }

    #[test]
    fn test_precedence() {
        // ab|c should be (ab)|c
        let _ = RegexAst::parse("ab|c").unwrap();
    }

    // Unicode escape tests

    #[test]
    fn test_unicode_escape_braced() {
        // \u{0041} should parse to 'A'
        let ast = RegexAst::parse("\\u{0041}").unwrap();
        assert_eq!(ast.root, Regex::Literal('A'));
    }

    #[test]
    fn test_unicode_escape_fixed() {
        // \u0042 should parse to 'B'
        let ast = RegexAst::parse("\\u0042").unwrap();
        assert_eq!(ast.root, Regex::Literal('B'));
    }

    #[test]
    fn test_unicode_escape_x_braced() {
        // \x{0043} should parse to 'C'
        let ast = RegexAst::parse("\\x{0043}").unwrap();
        assert_eq!(ast.root, Regex::Literal('C'));
    }

    #[test]
    fn test_unicode_escape_higher_codepoint() {
        // Greek alpha: U+03B1
        let ast = RegexAst::parse("\\u{03B1}").unwrap();
        assert_eq!(ast.root, Regex::Literal('\u{03B1}'));
    }

    #[test]
    fn test_unicode_escape_emoji() {
        // Smiling face: U+1F600
        let ast = RegexAst::parse("\\u{1F600}").unwrap();
        assert_eq!(ast.root, Regex::Literal('\u{1F600}'));
    }

    #[test]
    fn test_unicode_property_uppercase() {
        // \p{Lu} - uppercase letters
        let ast = RegexAst::parse("\\p{Lu}").unwrap();
        if let Regex::CharClass(cc) = ast.root {
            assert!(!cc.negated);
            assert!(!cc.unicode_ranges.is_empty());
        } else {
            panic!("Expected CharClass for \\p{{Lu}}");
        }
    }

    #[test]
    fn test_unicode_property_lowercase() {
        // \p{Ll} - lowercase letters
        let ast = RegexAst::parse("\\p{Ll}").unwrap();
        if let Regex::CharClass(cc) = ast.root {
            assert!(!cc.negated);
            assert!(!cc.unicode_ranges.is_empty());
        } else {
            panic!("Expected CharClass for \\p{{Ll}}");
        }
    }

    #[test]
    fn test_unicode_property_negated() {
        // \P{Lu} - NOT uppercase letters
        let ast = RegexAst::parse("\\P{Lu}").unwrap();
        if let Regex::CharClass(cc) = ast.root {
            assert!(cc.negated);
            assert!(!cc.unicode_ranges.is_empty());
        } else {
            panic!("Expected CharClass for \\P{{Lu}}");
        }
    }

    #[test]
    fn test_unicode_property_digit() {
        // \p{Nd} - decimal digit numbers
        let ast = RegexAst::parse("\\p{Nd}").unwrap();
        if let Regex::CharClass(cc) = ast.root {
            assert!(!cc.negated);
            assert!(!cc.unicode_ranges.is_empty());
        } else {
            panic!("Expected CharClass for \\p{{Nd}}");
        }
    }

    #[test]
    fn test_unicode_property_script() {
        // \p{Greek} or \p{Script=Greek}
        let ast = RegexAst::parse("\\p{Greek}").unwrap();
        if let Regex::CharClass(cc) = ast.root {
            assert!(!cc.negated);
            assert!(!cc.unicode_ranges.is_empty());
        } else {
            panic!("Expected CharClass for \\p{{Greek}}");
        }
    }

    #[test]
    fn test_unicode_property_script_with_prefix() {
        let ast = RegexAst::parse("\\p{Script=Latin}").unwrap();
        if let Regex::CharClass(cc) = ast.root {
            assert!(!cc.negated);
            assert!(!cc.unicode_ranges.is_empty());
        } else {
            panic!("Expected CharClass for \\p{{Script=Latin}}");
        }
    }

    #[test]
    fn test_unicode_in_concat() {
        // Mix Unicode escape with regular chars
        let ast = RegexAst::parse("a\\u{0042}c").unwrap();
        // Should be concat of 'a', 'B', 'c'
        match ast.root {
            Regex::Concat(_, _) => (),
            _ => panic!("Expected Concat"),
        }
    }

    #[test]
    fn test_unicode_property_in_alternation() {
        let ast = RegexAst::parse("\\p{Lu}|\\p{Ll}").unwrap();
        match ast.root {
            Regex::Union(_, _) => (),
            _ => panic!("Expected Union"),
        }
    }

    #[test]
    fn test_charclass_has_unicode_properties() {
        let cc = CharClass {
            negated: false,
            ranges: vec![],
            unicode_ranges: vec![0x41..=0x5A],
        };
        assert!(cc.has_unicode_properties());

        let cc_no_unicode = CharClass {
            negated: false,
            ranges: vec![CharRange::Single('a')],
            unicode_ranges: vec![],
        };
        assert!(!cc_no_unicode.has_unicode_properties());
    }

    #[test]
    fn test_charclass_matches_unicode() {
        let cc = CharClass {
            negated: false,
            ranges: vec![],
            unicode_ranges: vec![0x41..=0x5A], // A-Z
        };

        assert!(cc.matches('A'));
        assert!(cc.matches('M'));
        assert!(cc.matches('Z'));
        assert!(!cc.matches('a'));
        assert!(!cc.matches('0'));
    }

    #[test]
    fn test_charclass_matches_negated_unicode() {
        let cc = CharClass {
            negated: true,
            ranges: vec![],
            unicode_ranges: vec![0x41..=0x5A], // NOT A-Z
        };

        assert!(!cc.matches('A'));
        assert!(!cc.matches('Z'));
        assert!(cc.matches('a'));
        assert!(cc.matches('0'));
    }

    #[test]
    fn test_charclass_expand_unicode() {
        let cc = CharClass {
            negated: false,
            ranges: vec![],
            unicode_ranges: vec![0x41..=0x43], // A, B, C
        };

        let chars = cc.expand_unicode(0x7F);
        assert_eq!(chars, vec!['A', 'B', 'C']);
    }

    #[test]
    fn test_escape_d() {
        let ast = RegexAst::parse("\\d").unwrap();
        if let Regex::CharClass(cc) = ast.root {
            assert!(!cc.negated);
            assert!(cc.matches('0'));
            assert!(cc.matches('9'));
            assert!(!cc.matches('a'));
        } else {
            panic!("Expected CharClass for \\d");
        }
    }

    #[test]
    fn test_escape_d_negated() {
        let ast = RegexAst::parse("\\D").unwrap();
        if let Regex::CharClass(cc) = ast.root {
            assert!(cc.negated);
            // Negated: matches non-digits
        } else {
            panic!("Expected CharClass for \\D");
        }
    }

    #[test]
    fn test_escape_w() {
        let ast = RegexAst::parse("\\w").unwrap();
        if let Regex::CharClass(cc) = ast.root {
            assert!(!cc.negated);
            assert!(cc.matches('a'));
            assert!(cc.matches('Z'));
            assert!(cc.matches('0'));
            assert!(cc.matches('_'));
        } else {
            panic!("Expected CharClass for \\w");
        }
    }

    #[test]
    fn test_escape_s() {
        let ast = RegexAst::parse("\\s").unwrap();
        if let Regex::CharClass(cc) = ast.root {
            assert!(!cc.negated);
            assert!(cc.matches(' '));
            assert!(cc.matches('\t'));
            assert!(cc.matches('\n'));
        } else {
            panic!("Expected CharClass for \\s");
        }
    }

    #[test]
    fn test_invalid_unicode_escape() {
        // Invalid: \u without 4 hex digits
        let result = RegexAst::parse("\\u12");
        assert!(result.is_err());
    }

    #[test]
    fn test_invalid_unicode_property() {
        // Invalid property name
        let result = RegexAst::parse("\\p{InvalidProperty}");
        assert!(result.is_err());
    }

    #[test]
    fn test_unclosed_unicode_property() {
        // Missing closing brace
        let result = RegexAst::parse("\\p{Lu");
        assert!(result.is_err());
    }
}
