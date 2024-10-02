use crate::scanner::token::Token;
use crate::{
    chunk::{Chunk, Operation},
    scanner::{token::TokenKind, Scanner},
    value::Value,
};

mod precedence;

pub fn compile(source: &str) -> Option<Chunk> {
    let mut compiler = Compiler::new(source);
    compiler.compile()
}

pub struct Compiler<'source> {
    compiling_chunk: Chunk,
    scanner: Scanner<'source>,
    previous: Token<'source>,
    current: Token<'source>,
    had_error: bool,
    panic_mode: bool,
}

impl<'source> Compiler<'source> {
    pub fn new(source: &'source str) -> Self {
        let initial_token = Token::initial();
        let chunk = Chunk::new();
        Self {
            compiling_chunk: chunk,
            scanner: Scanner::new(source),
            previous: initial_token,
            current: initial_token,
            had_error: false,
            panic_mode: false,
        }
    }

    pub fn compile(&mut self) -> Option<Chunk> {
        self.advance();
        while !self.r#match(TokenKind::Eof) {
            self.declaration();
        }
        self.end_compiler();
        match self.had_error {
            true => None,
            false => Some(self.compiling_chunk.clone()),
        }
    }

    fn r#match(&mut self, token_kind: TokenKind) -> bool {
        if !self.check_current_token(token_kind) {
            return false;
        }
        self.advance();
        true
    }

    fn check_current_token(&self, token_kind: TokenKind) -> bool {
        self.current.kind == token_kind
    }

    fn declaration(&mut self) {
        self.statement()
    }

    fn statement(&mut self) {
        if self.r#match(TokenKind::Print) {
            self.print_statement();
        }
    }

    fn print_statement(&mut self) {
        self.expression();
        self.consume(TokenKind::Semicolon, "Expect ';' after value.");
        self.emit_operation(Operation::Print);
    }

    fn advance(&mut self) {
        self.previous = self.current;
        loop {
            self.current = self.scanner.scan_token();
            if self.current.kind != TokenKind::Error {
                break;
            }
            self.error_at_current(self.current.lexeme);
        }
    }

    fn consume(&mut self, token_kind: TokenKind, message: &str) {
        if self.current.kind == token_kind {
            self.advance();
            return;
        }
        self.error_at_current(message);
    }

    fn emit_operation(&mut self, operation: Operation) {
        self.compiling_chunk.write(operation, self.previous.line);
    }

    fn end_compiler(&mut self) {
        self.emit_operation(Operation::Return);
        if cfg!(feature = "debug-print-code") {
            self.compiling_chunk.disassemble();
        }
    }

    fn emit_constant(&mut self, constant: Value) {
        if let Some(constant_index) = self.compiling_chunk.add_constant(constant) {
            self.emit_operation(Operation::Constant(constant_index));
        }
    }

    fn error(&mut self, message: &str) {
        self.error_at(self.previous, message);
    }

    fn error_at_current(&mut self, message: &str) {
        self.error_at(self.current, message);
    }

    fn error_at(&mut self, token: Token, message: &str) {
        if self.panic_mode {
            return;
        }
        self.panic_mode = true;
        eprint!("[line {}] Error", token.line);

        match token.kind {
            TokenKind::Eof => eprint!(" at end"),
            TokenKind::Error => (),
            _ => eprint!(" at '{}'", token.lexeme),
        }
        eprintln!(": {message}");
        self.had_error = true;
    }
}
