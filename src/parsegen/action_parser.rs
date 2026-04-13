use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    Dollar,
    LessThan,
    GreaterThan,
    Number(usize),
    At,
    Ident(String),
    StringLit(String),
    LBrace,
    RBrace,
    LParen,
    RParen,
    Semicolon,
    Printf,
    Yyerrok,
    Yyclearin,
    Yyerror,
    Other(char),
}

pub struct Tokenizer {
    input: Vec<char>,
    pos: usize,
}

impl Tokenizer {
    pub fn new(input: &str) -> Self {
        Tokenizer {
            input: input.chars().collect(),
            pos: 0,
        }
    }

    pub fn peek(&self) -> Option<char> {
        if self.pos < self.input.len() {
            Some(self.input[self.pos])
        } else {
            None
        }
    }

    pub fn peek_ahead(&self, n: usize) -> Option<char> {
        if self.pos + n < self.input.len() {
            Some(self.input[self.pos + n])
        } else {
            None
        }
    }

    pub fn advance(&mut self) {
        if self.pos < self.input.len() {
            self.pos += 1;
        }
    }

    pub fn is_eof(&self) -> bool {
        self.pos >= self.input.len()
    }

    pub fn skip_whitespace(&mut self) {
        while self.pos < self.input.len() && self.input[self.pos].is_whitespace() {
            self.pos += 1;
        }
    }

    fn read_string(&mut self, quote: char) -> String {
        let mut result = String::new();
        self.advance();

        while self.pos < self.input.len() && self.input[self.pos] != quote {
            if self.input[self.pos] == '\\' && self.pos + 1 < self.input.len() {
                result.push(self.input[self.pos]);
                self.advance();
                result.push(self.input[self.pos]);
                self.advance();
            } else {
                result.push(self.input[self.pos]);
                self.advance();
            }
        }

        if self.pos < self.input.len() {
            self.advance();
        }
        result
    }

    fn read_ident(&mut self) -> String {
        let mut result = String::new();
        while self.pos < self.input.len()
            && (self.input[self.pos].is_alphanumeric() || self.input[self.pos] == '_') {
            result.push(self.input[self.pos]);
            self.advance();
        }
        result
    }

    fn read_number(&mut self) -> usize {
        let mut result = String::new();
        while self.pos < self.input.len() && self.input[self.pos].is_ascii_digit() {
            result.push(self.input[self.pos]);
            self.advance();
        }
        result.parse().unwrap_or(0)
    }

    pub fn next_token(&mut self) -> Option<Token> {
        self.skip_whitespace();

        if self.is_eof() {
            return None;
        }

        match self.input[self.pos] {
            '$' => {
                self.advance();
                Some(Token::Dollar)
            }
            '@' => {
                self.advance();
                Some(Token::At)
            }
            '<' => {
                self.advance();
                Some(Token::LessThan)
            }
            '>' => {
                self.advance();
                Some(Token::GreaterThan)
            }
            '{' => {
                self.advance();
                Some(Token::LBrace)
            }
            '}' => {
                self.advance();
                Some(Token::RBrace)
            }
            '(' => {
                self.advance();
                Some(Token::LParen)
            }
            ')' => {
                self.advance();
                Some(Token::RParen)
            }
            ';' => {
                self.advance();
                Some(Token::Semicolon)
            }
            '"' => {
                let s = self.read_string('"');
                Some(Token::StringLit(s))
            }
            '\'' => {
                let s = self.read_string('\'');
                Some(Token::StringLit(s))
            }
            '/' if self.peek_ahead(1) == Some('/') => {
                self.advance();
                self.advance();
                while self.pos < self.input.len() && self.input[self.pos] != '\n' {
                    self.advance();
                }
                self.next_token()
            }
            '/' if self.peek_ahead(1) == Some('*') => {
                self.advance();
                self.advance();
                while self.pos + 1 < self.input.len() {
                    if self.input[self.pos] == '*' && self.input[self.pos + 1] == '/' {
                        self.advance();
                        self.advance();
                        break;
                    }
                    self.advance();
                }
                self.next_token()
            }
            c if c.is_ascii_digit() => {
                let n = self.read_number();
                Some(Token::Number(n))
            }
            c if c.is_alphabetic() || c == '_' => {
                let ident = self.read_ident();
                match ident.as_str() {
                    "printf" => Some(Token::Printf),
                    "yyerrok" => Some(Token::Yyerrok),
                    "yyclearin" => Some(Token::Yyclearin),
                    "yyerror" => Some(Token::Yyerror),
                    _ => Some(Token::Ident(ident)),
                }
            }
            c => {
                self.advance();
                Some(Token::Other(c))
            }
        }
    }
}

pub fn extract_action_block(input: &str) -> String {
    let chars: Vec<char> = input.chars().collect();
    let mut depth = 0;
    let mut start = None;
    let mut end = 0;

    for (i, &c) in chars.iter().enumerate() {
        match c {
            '{' => {
                if start.is_none() {
                    start = Some(i + 1);
                }
                depth += 1;
            }
            '}' => {
                depth -= 1;
                if depth == 0 && start.is_some() {
                    end = i;
                    break;
                }
            }
            '"' | '\'' => {
                let mut j = i + 1;
                while j < chars.len() && chars[j] != c {
                    if chars[j] == '\\' && j + 1 < chars.len() {
                        j += 2;
                    } else {
                        j += 1;
                    }
                }
            }
            _ => {}
        }
    }

    if let Some(start_pos) = start {
        if end > start_pos {
            chars[start_pos..end].iter().collect()
        } else {
            input.to_string()
        }
    } else {
        input.to_string()
    }
}

pub struct SemanticAction {
    pub original: String,
    pub tokens: Vec<Token>,
    pub is_multiline: bool,
    pub has_control_flow: bool,
}

impl SemanticAction {
    pub fn parse(input: &str) -> Self {
        let is_multiline = input.lines().count() > 1;
        let has_control_flow = input.contains("if") || input.contains("for") || input.contains("while");

        let mut tokenizer = Tokenizer::new(input);
        let mut tokens = Vec::new();

        while let Some(token) = tokenizer.next_token() {
            tokens.push(token);
        }

        SemanticAction {
            original: input.to_string(),
            tokens,
            is_multiline,
            has_control_flow,
        }
    }

    pub fn substitute_refs(&self, rhs_len: usize) -> HashMap<String, String> {
        let mut refs = HashMap::new();
        let mut i = 0;

        while i < self.tokens.len() {
            if self.tokens[i] == Token::Dollar && i + 1 < self.tokens.len() {
                match &self.tokens[i + 1] {
                    Token::Dollar => {
                        refs.insert("$$".to_string(), "$$".to_string());
                        i += 2;
                    }
                    Token::Number(n) if *n > 0 && *n <= rhs_len => {
                        let key = format!("${}", n);
                        refs.insert(key, format!("${}", n));
                        i += 2;
                    }
                    Token::LessThan if i + 2 < self.tokens.len() => {
                        if let Token::Ident(typ) = &self.tokens[i + 2] {
                            if i + 3 < self.tokens.len() && self.tokens[i + 3] == Token::GreaterThan {
                                if i + 4 < self.tokens.len() {
                                    match &self.tokens[i + 4] {
                                        Token::Dollar => {
                                            let key = format!("$<{}>${}", typ, typ);
                                            refs.insert(key, format!("$<{}>$", typ));
                                            i += 5;
                                        }
                                        Token::Number(n) if *n > 0 && *n <= rhs_len => {
                                            let key = format!("$<{}>{}", typ, n);
                                            refs.insert(key.clone(), format!("$<{}>$", typ));
                                            i += 5;
                                        }
                                        _ => {
                                            i += 1;
                                        }
                                    }
                                } else {
                                    i += 1;
                                }
                            } else {
                                i += 1;
                            }
                        } else {
                            i += 1;
                        }
                    }
                    Token::At if i + 2 < self.tokens.len() => {
                        match &self.tokens[i + 2] {
                            Token::Dollar => {
                                refs.insert("@$".to_string(), "@$".to_string());
                                i += 3;
                            }
                            Token::Number(n) if *n > 0 && *n <= rhs_len => {
                                let key = format!("@{}", n);
                                refs.insert(key, format!("@{}", n));
                                i += 3;
                            }
                            _ => {
                                i += 1;
                            }
                        }
                    }
                    _ => {
                        i += 1;
                    }
                }
            } else {
                i += 1;
            }
        }

        refs
    }
}
