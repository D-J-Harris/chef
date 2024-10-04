use std::u8;

use crate::scanner::token::Token;
use crate::{
    chunk::{Chunk, Operation},
    scanner::{token::TokenKind, Scanner},
    value::Value,
};

mod precedence;

#[derive(Debug)]
struct Local<'source> {
    depth: Option<u8>,
    name: &'source str,
}

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
    scope_depth: u8,
    locals: Vec<Local<'source>>,
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
            scope_depth: 0,
            locals: Vec::with_capacity(u8::MAX as usize),
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
        if self.r#match(TokenKind::Var) {
            self.var_declaration();
        } else {
            self.statement();
        }
        if self.panic_mode {
            self.synchronise();
        }
    }

    fn var_declaration(&mut self) {
        let Some(constant_index) = self.parse_variable("Expect variable name.") else {
            self.error("Reached constant limit.");
            return;
        };
        if self.r#match(TokenKind::Equal) {
            self.expression();
        } else {
            self.emit_operation(Operation::Nil);
        }
        self.consume(TokenKind::Semicolon, "Expect ';' after value.");
        self.define_variable(constant_index);
    }

    fn parse_variable(&mut self, error_message: &str) -> Option<u8> {
        self.consume(TokenKind::Identifier, error_message);
        self.declare_variable();
        if self.scope_depth > 0 {
            // Return dummy variable since locals aren't looked up by name at runntime
            return Some(0);
        }
        self.constant_identifier(&self.previous.lexeme)
    }

    fn declare_variable(&mut self) {
        if self.scope_depth == 0 {
            return;
        }
        let variable_name = self.previous.lexeme;

        // Detect clashing variable names in current scope (does not include shadowing, which is allowed).
        let mut has_match_name_error = false;
        for local in self.locals.iter().rev() {
            if let Some(depth) = local.depth {
                if depth < self.scope_depth {
                    break;
                }
            }
            if local.name == variable_name {
                has_match_name_error = true
            }
        }
        if has_match_name_error {
            self.error("Already a variable with this name in this scope.");
        }
        self.add_local(variable_name)
    }

    fn add_local(&mut self, name: &'source str) {
        // "Declare" depth with None, it will later be initialised when variable defined
        self.locals.push(Local { depth: None, name });
    }

    fn constant_identifier(&mut self, token_name: &str) -> Option<u8> {
        self.compiling_chunk
            .add_constant(Value::String(token_name.into()))
    }

    fn define_variable(&mut self, constant_index: u8) {
        if self.scope_depth > 0 {
            // Mark as initialised
            let Some(local) = self.locals.last_mut() else {
                self.error("No local scopes defined.");
                return;
            };
            local.depth = Some(self.scope_depth);
            return;
        }
        self.emit_operation(Operation::DefineGlobal(constant_index));
    }

    fn statement(&mut self) {
        if self.r#match(TokenKind::Print) {
            self.print_statement();
        } else if self.r#match(TokenKind::For) {
            self.for_statement();
        } else if self.r#match(TokenKind::If) {
            self.if_statement();
        } else if self.r#match(TokenKind::While) {
            self.while_statement();
        } else if self.r#match(TokenKind::LeftBrace) {
            self.begin_scope();
            self.block();
            self.end_scope();
        } else {
            self.expression_statement();
        }
    }

    fn begin_scope(&mut self) {
        self.scope_depth += 1;
    }

    fn end_scope(&mut self) {
        self.scope_depth -= 1;
        let mut emit_count = 0;
        for local in self.locals.iter().rev() {
            if let Some(local_depth) = local.depth {
                if local_depth <= self.scope_depth {
                    break;
                }
            }
            emit_count += 1;
        }
        for _ in 0..emit_count {
            self.locals.pop();
            self.emit_operation(Operation::Pop);
        }
    }

    fn block(&mut self) {
        while !self.check_current_token(TokenKind::RightBrace)
            && !self.check_current_token(TokenKind::Eof)
        {
            self.declaration();
        }
        self.consume(TokenKind::RightBrace, "Expect '}' after block.");
    }

    fn print_statement(&mut self) {
        self.expression();
        self.consume(TokenKind::Semicolon, "Expect ';' after value.");
        self.emit_operation(Operation::Print);
    }

    fn if_statement(&mut self) {
        self.consume(TokenKind::LeftParen, "Expect '(' after 'if'.");
        self.expression();
        self.consume(TokenKind::RightParen, "Expect ')' after condition.");

        self.emit_operation(Operation::JumpIfFalse(u8::MAX));
        let num_operations_if = self.compiling_chunk.code.len();
        self.emit_operation(Operation::Pop);
        self.statement();
        self.emit_operation(Operation::Jump(u8::MAX));
        let num_operations_else = self.compiling_chunk.code.len();
        self.patch_jump(num_operations_if);
        self.emit_operation(Operation::Pop);
        if self.r#match(TokenKind::Else) {
            self.statement();
        }
        self.patch_jump(num_operations_else);
    }

    fn patch_jump(&mut self, num_operations_before: usize) {
        let num_operations_after = self.compiling_chunk.code.len();
        if num_operations_after - num_operations_before > u8::MAX as usize {
            self.error("Too much code to jump over.");
            return;
        }
        match self.compiling_chunk.code.get_mut(num_operations_before - 1) {
            Some(Operation::JumpIfFalse(jump)) | Some(Operation::Jump(jump)) => {
                *jump = (num_operations_after - num_operations_before) as u8
            }
            _ => {
                self.error("Could not find reference to added jump_if_false operation.");
                return;
            }
        }
    }

    fn while_statement(&mut self) {
        let num_operations_loop_start = self.compiling_chunk.code.len();
        self.consume(TokenKind::LeftParen, "Expect '(' after 'while'.");
        self.expression();
        self.consume(TokenKind::RightParen, "Expect ')' after condition.");

        self.emit_operation(Operation::JumpIfFalse(u8::MAX));
        let num_operations_exit = self.compiling_chunk.code.len();
        self.emit_operation(Operation::Pop);
        self.statement();
        self.emit_loop(num_operations_loop_start);

        self.patch_jump(num_operations_exit);
        self.emit_operation(Operation::Pop);
    }

    fn for_statement(&mut self) {
        self.begin_scope();
        self.consume(TokenKind::LeftParen, "Expect '(' after 'for'.");
        if self.r#match(TokenKind::Semicolon) {
            // No initialiser
        } else if self.r#match(TokenKind::Var) {
            self.var_declaration();
        } else {
            self.expression_statement();
        }
        let mut num_operations_loop_start = self.compiling_chunk.code.len();

        // Condition clause.
        let mut num_operations_exit = None;
        if !self.r#match(TokenKind::Semicolon) {
            self.expression();
            self.consume(TokenKind::Semicolon, "Expect ';' after loop condition.");
            // Jump out of the loop if the condition is false.
            self.emit_operation(Operation::JumpIfFalse(u8::MAX));
            num_operations_exit = Some(self.compiling_chunk.code.len());
            self.emit_operation(Operation::Pop);
        }

        // Incremenet clause.
        if !self.r#match(TokenKind::RightParen) {
            self.emit_operation(Operation::Jump(u8::MAX));
            let num_operations_jump = self.compiling_chunk.code.len();
            let num_operations_increment_start = self.compiling_chunk.code.len();
            self.expression();
            self.emit_operation(Operation::Pop);
            self.consume(TokenKind::RightParen, "Expect ')' after for clauses.");
            self.emit_loop(num_operations_loop_start);
            num_operations_loop_start = num_operations_increment_start;
            self.patch_jump(num_operations_jump);
        }

        self.statement();
        self.emit_loop(num_operations_loop_start);

        // Patch exit loop jump from condition clause.
        if let Some(num_operations_exit) = num_operations_exit {
            self.patch_jump(num_operations_exit);
            self.emit_operation(Operation::Pop);
        }
        self.end_scope();
    }

    fn emit_loop(&mut self, num_operations_loop_start: usize) {
        let offset = self.compiling_chunk.code.len() - num_operations_loop_start;
        if offset > u8::MAX as usize {
            self.error("Too much code to jump over.");
            return;
        }
        self.emit_operation(Operation::Loop(offset as u8));
    }

    fn expression_statement(&mut self) {
        self.expression();
        self.consume(TokenKind::Semicolon, "Expect ';' after expression.");
        self.emit_operation(Operation::Pop);
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

    fn synchronise(&mut self) {
        self.panic_mode = false;
        while self.current.kind != TokenKind::Eof {
            if self.previous.kind == TokenKind::Semicolon {
                return;
            }
            match self.current.kind {
                TokenKind::Class
                | TokenKind::Fun
                | TokenKind::Var
                | TokenKind::For
                | TokenKind::If
                | TokenKind::While
                | TokenKind::Print
                | TokenKind::Return => return,
                _ => (),
            }
            self.advance();
        }
    }
}
