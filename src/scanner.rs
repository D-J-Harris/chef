use std::collections::HashMap;

use crate::common::SUPER_STRING;

pub struct Scanner<'source> {
    identifiers: HashMap<&'static str, TokenKind>,
    source: &'source str,
    start: usize,
    current: usize,
    line: usize,
}

impl<'source> Scanner<'source> {
    pub fn new(source: &'source str) -> Self {
        let mut identifiers = HashMap::with_capacity(16);
        identifiers.insert(SUPER_STRING, TokenKind::Super);
        identifiers.insert("and", TokenKind::And);
        identifiers.insert("class", TokenKind::Class);
        identifiers.insert("else", TokenKind::Else);
        identifiers.insert("false", TokenKind::False);
        identifiers.insert("for", TokenKind::For);
        identifiers.insert("fun", TokenKind::Fun);
        identifiers.insert("if", TokenKind::If);
        identifiers.insert("nil", TokenKind::Nil);
        identifiers.insert("or", TokenKind::Or);
        identifiers.insert("print", TokenKind::Print);
        identifiers.insert("return", TokenKind::Return);
        identifiers.insert("this", TokenKind::This);
        identifiers.insert("true", TokenKind::True);
        identifiers.insert("var", TokenKind::Var);
        identifiers.insert("while", TokenKind::While);
        Self {
            identifiers,
            source,
            start: 0,
            current: 0,
            line: 1,
        }
    }

    fn advance(&mut self) -> u8 {
        let byte = self.source.as_bytes()[self.current];
        self.current += 1;
        byte
    }

    fn is_at_end(&self) -> bool {
        self.peek() == b'\0'
    }

    pub fn scan_token(&mut self) -> Token<'source> {
        self.skip_whitespace();
        self.start = self.current;
        if self.is_at_end() {
            return self.make_token(TokenKind::Eof);
        }
        let byte = self.advance();
        match byte {
            b'(' => self.make_token(TokenKind::LeftParen),
            b')' => self.make_token(TokenKind::RightParen),
            b'{' => self.make_token(TokenKind::LeftBrace),
            b'}' => self.make_token(TokenKind::RightBrace),
            b';' => self.make_token(TokenKind::Semicolon),
            b',' => self.make_token(TokenKind::Comma),
            b'.' => self.make_token(TokenKind::Dot),
            b'-' => self.make_token(TokenKind::Minus),
            b'+' => self.make_token(TokenKind::Plus),
            b'/' => self.make_token(TokenKind::Slash),
            b'*' => self.make_token(TokenKind::Star),
            b'!' => match self.is_match(b'=') {
                true => self.make_token(TokenKind::BangEqual),
                false => self.make_token(TokenKind::Bang),
            },
            b'=' => match self.is_match(b'=') {
                true => self.make_token(TokenKind::EqualEqual),
                false => self.make_token(TokenKind::Equal),
            },
            b'<' => match self.is_match(b'=') {
                true => self.make_token(TokenKind::LessEqual),
                false => self.make_token(TokenKind::Less),
            },
            b'>' => match self.is_match(b'=') {
                true => self.make_token(TokenKind::GreaterEqual),
                false => self.make_token(TokenKind::Greater),
            },
            b'"' => self.make_string_token(),
            b if b.is_ascii_digit() => self.make_number_token(),
            b if is_alpha(b) => self.make_identifier_token(),
            _ => unimplemented!(),
        }
    }

    fn lexeme(&self) -> &'source str {
        &self.source[self.start..self.current]
    }

    fn make_token(&self, kind: TokenKind) -> Token<'source> {
        Token {
            kind,
            lexeme: self.lexeme(),
            line: self.line,
        }
    }

    fn make_error_token(&self, message: &'static str) -> Token<'source> {
        Token {
            kind: TokenKind::Error,
            lexeme: message,
            line: self.line,
        }
    }

    fn make_string_token(&mut self) -> Token<'source> {
        while self.peek() != b'"' && !self.is_at_end() {
            if self.advance() == b'\n' {
                self.line += 1
            }
        }
        if self.is_at_end() {
            return self.make_error_token("Unterminated string.");
        }
        self.current += 1;
        self.make_token(TokenKind::String)
    }

    fn make_number_token(&mut self) -> Token<'source> {
        while self.peek().is_ascii_digit() {
            self.current += 1
        }
        let Some(next) = self.peek_next() else {
            return self.make_token(TokenKind::Number);
        };
        if self.peek() == b'.' && next.is_ascii_digit() {
            self.current += 1;
            while self.peek().is_ascii_digit() {
                self.current += 1
            }
        }
        self.make_token(TokenKind::Number)
    }

    fn make_identifier_token(&mut self) -> Token<'source> {
        loop {
            let byte = self.peek();
            if !byte.is_ascii_digit() && !is_alpha(byte) {
                break;
            }
            self.current += 1;
        }
        let kind = match self.identifiers.get(self.lexeme()) {
            Some(kind) => *kind,
            None => TokenKind::Identifier,
        };
        self.make_token(kind)
    }

    fn is_match(&mut self, byte: u8) -> bool {
        if self.is_at_end() {
            return false;
        }
        if self.peek() != byte {
            return false;
        }
        self.current += 1;
        true
    }

    fn peek(&self) -> u8 {
        self.source.as_bytes()[self.current]
    }

    fn peek_next(&self) -> Option<u8> {
        match self.current + 1 < self.source.len() {
            true => Some(self.source.as_bytes()[self.current + 1]),
            false => None,
        }
    }

    fn skip_whitespace(&mut self) {
        loop {
            let byte = self.peek();
            match byte {
                b' ' | b'\r' | b'\t' => self.current += 1,
                b'\n' => {
                    self.line += 1;
                    self.current += 1;
                }
                b'/' => match self.peek_next() {
                    Some(b'/') => {
                        while self.peek() != b'\n' && !self.is_at_end() {
                            self.current += 1
                        }
                    }
                    Some(_) | None => return,
                },
                _ => break,
            }
        }
    }
}

fn is_alpha(byte: u8) -> bool {
    byte.is_ascii_lowercase() || byte.is_ascii_uppercase() || byte == b'_'
}

#[derive(Debug, PartialEq, Eq, Clone, Copy, Hash)]
pub enum TokenKind {
    // Single-character tokens.
    LeftParen,
    RightParen,
    LeftBrace,
    RightBrace,
    Comma,
    Dot,
    Minus,
    Plus,
    Semicolon,
    Slash,
    Star,
    // One or two character tokens.
    Bang,
    BangEqual,
    Equal,
    EqualEqual,
    Greater,
    GreaterEqual,
    Less,
    LessEqual,
    // Literals.
    Identifier,
    String,
    Number,
    // Keywords.
    And,
    Class,
    Else,
    False,
    For,
    Fun,
    If,
    Nil,
    Or,
    Print,
    Return,
    Super,
    This,
    True,
    Var,
    While,
    // Other.
    Error,
    Eof,
}

#[derive(Debug, Clone, Copy)]
pub struct Token<'source> {
    pub kind: TokenKind,
    pub lexeme: &'source str,
    pub line: usize,
}

impl<'source> Token<'source> {
    pub fn new(lexeme: &'source str, line: usize, kind: TokenKind) -> Self {
        Self { kind, lexeme, line }
    }
}
