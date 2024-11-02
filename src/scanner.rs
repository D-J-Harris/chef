use std::collections::HashMap;
pub struct Scanner<'src> {
    identifiers: HashMap<&'static str, TokenKind>,
    source: &'src str,
    start: usize,
    current: usize,
    line: usize,
}

impl<'src> Scanner<'src> {
    pub fn new(source: &'src str) -> Self {
        let mut identifiers = HashMap::new();
        identifiers.insert("compliments", TokenKind::And);
        identifiers.insert("and", TokenKind::ParameterAnd);
        identifiers.insert("add", TokenKind::Plus);
        identifiers.insert("now", TokenKind::BareFunctionInvocation);
        identifiers.insert("minus", TokenKind::Minus);
        identifiers.insert("check", TokenKind::If);
        identifiers.insert("with", TokenKind::With);
        identifiers.insert("not", TokenKind::Bang);
        identifiers.insert("isnt", TokenKind::BangEqual);
        identifiers.insert("split", TokenKind::Slash);
        identifiers.insert("multiply", TokenKind::Star);
        identifiers.insert("above", TokenKind::Greater);
        identifiers.insert("below", TokenKind::Less);
        identifiers.insert("otherwise", TokenKind::Else);
        identifiers.insert("false", TokenKind::False);
        identifiers.insert("nil", TokenKind::Nil);
        identifiers.insert("or", TokenKind::Or);
        identifiers.insert("is", TokenKind::EqualEqual);
        identifiers.insert("to", TokenKind::Equal);
        identifiers.insert("set", TokenKind::Var);
        identifiers.insert("taste", TokenKind::Print);
        identifiers.insert("serve", TokenKind::Return);
        identifiers.insert("true", TokenKind::True);
        identifiers.insert("while", TokenKind::While);
        identifiers.insert("end", TokenKind::RightBrace);
        identifiers.insert("Recipe", TokenKind::Recipe);
        identifiers.insert("Ingredients", TokenKind::Ingredients);
        identifiers.insert("Utensils", TokenKind::Utensils);
        identifiers.insert("Steps", TokenKind::Steps);

        identifiers.insert("x", TokenKind::ParameterIdent);
        identifiers.insert("y", TokenKind::ParameterIdent);
        identifiers.insert("z", TokenKind::ParameterIdent);

        identifiers.insert("egg", TokenKind::VarIdent);
        identifiers.insert("flour", TokenKind::VarIdent);
        identifiers.insert("sugar", TokenKind::VarIdent);
        identifiers.insert("milk", TokenKind::VarIdent);
        identifiers.insert("chocolate", TokenKind::VarIdent);
        identifiers.insert("banana", TokenKind::VarIdent);

        identifiers.insert("whisk", TokenKind::FunIdent);
        identifiers.insert("bake", TokenKind::FunIdent);
        identifiers.insert("cook", TokenKind::FunIdent);
        identifiers.insert("time", TokenKind::FunIdent);

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

    pub fn scan_token(&mut self) -> Token<'src> {
        self.skip_whitespace();
        self.start = self.current;
        if self.is_at_end() {
            return self.make_token(TokenKind::Eof);
        }
        let byte = self.advance();
        match byte {
            b',' => self.make_token(TokenKind::Comma),
            b'.' => self.make_token(TokenKind::Dot),
            b'(' => self.make_token(TokenKind::LeftParen),
            b')' => self.make_token(TokenKind::RightParen),
            b'"' => self.make_string_token(),
            b if b.is_ascii_digit() => self.make_number_token(),
            b if is_alpha(b) => self.make_identifier_token(),
            _ => self.make_error_token("Unexpected character."),
        }
    }

    fn lexeme(&self) -> &'src str {
        &self.source[self.start..self.current]
    }

    fn make_token(&self, kind: TokenKind) -> Token<'src> {
        Token {
            kind,
            lexeme: self.lexeme(),
            line: self.line,
        }
    }

    fn make_error_token(&self, message: &'static str) -> Token<'src> {
        Token {
            kind: TokenKind::Error,
            lexeme: message,
            line: self.line,
        }
    }

    fn make_identifier_token(&mut self) -> Token<'src> {
        loop {
            let byte = self.peek();
            if !byte.is_ascii_digit() && !is_alpha(byte) {
                break;
            }
            self.current += 1;
        }
        match self.identifiers.get(self.lexeme()) {
            Some(kind) => self.make_token(*kind),
            None => self.make_error_token("Invalid identifier."),
        }
    }

    fn make_string_token(&mut self) -> Token<'src> {
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

    fn make_number_token(&mut self) -> Token<'src> {
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
    RightBrace,
    Comma,
    Minus,
    Plus,
    Dot,
    Slash,
    Star,
    // One or two character tokens.
    Bang,
    BangEqual,
    Equal,
    EqualEqual,
    Greater,
    Less,
    // Literals.
    VarIdent,
    FunIdent,
    ParameterIdent,
    String,
    Number,
    // Keywords.
    And,
    Else,
    False,
    With,
    If,
    Nil,
    Or,
    Var,
    Print,
    Return,
    True,
    While,
    ParameterAnd,
    Recipe,
    Ingredients,
    Utensils,
    Steps,
    BareFunctionInvocation,
    // Other.
    Error,
    Eof,
}

#[derive(Debug, Clone, Copy)]
pub struct Token<'src> {
    pub kind: TokenKind,
    pub lexeme: &'src str,
    pub line: usize,
}

impl<'src> Token<'src> {
    pub fn new(lexeme: &'src str, line: usize, kind: TokenKind) -> Self {
        Self { kind, lexeme, line }
    }
}
