use std::rc::Rc;
use std::u8;

use crate::chunk::Opcode;
use crate::common::{FUNCTION_ARITY_MAX_COUNT, LOCALS_MAX_COUNT};
use crate::function::{Function, FunctionKind};
use crate::rules::{ParseFunctionKind, Precedence};
use crate::scanner::{Token, TokenKind};
use crate::value::Value;
use crate::{chunk::Code, scanner::Scanner};

pub struct Compiler<'src> {
    scanner: Scanner<'src>,
    previous: Token<'src>,
    current: Token<'src>,
    context: CompilerContext<'src>,
    had_error: bool,
    panic_mode: bool,
}

impl<'src> Compiler<'src> {
    pub fn new(source: &'src str) -> Self {
        let initial_token = Token::new("", 1, TokenKind::Error);
        let context = CompilerContext::new("", FunctionKind::Script);
        Self {
            scanner: Scanner::new(source),
            previous: initial_token,
            current: initial_token,
            had_error: false,
            panic_mode: false,
            context,
        }
    }

    fn current_chunk(&self) -> &Code {
        &self.context.function.chunk
    }

    fn init_compiler(&mut self, function_kind: FunctionKind) {
        let name = self.previous.lexeme;
        let compiler_context = CompilerContext::new(name, function_kind);
        let enclosing_compiler_context = std::mem::replace(&mut self.context, compiler_context);
        self.context.enclosing = Some(Box::new(enclosing_compiler_context));
    }

    fn end_compiler(&mut self) -> Option<Function> {
        self.emit_return();
        #[cfg(feature = "debug_code")]
        self.context.debug();
        self.context.enclosing.take().map(|parent| {
            let context = std::mem::replace(&mut self.context, *parent);
            context.function
        })
    }

    pub fn compile(mut self) -> Option<Rc<Function>> {
        self.advance();
        while !self.r#match(TokenKind::Eof) {
            self.declaration();
        }
        let had_error = self.had_error;
        let function = match self.end_compiler() {
            Some(f) => f,
            None => self.context.function,
        };
        match had_error {
            true => None,
            false => Some(Rc::new(function)),
        }
    }

    fn r#match(&mut self, token_kind: TokenKind) -> bool {
        if !self.check(token_kind) {
            return false;
        }
        self.advance();
        true
    }

    fn check(&self, token_kind: TokenKind) -> bool {
        self.current.kind == token_kind
    }

    fn declaration(&mut self) {
        if self.r#match(TokenKind::Fun) {
            self.fun_declaration();
        } else if self.r#match(TokenKind::Var) {
            self.var_declaration();
        } else {
            self.statement();
        }
        if self.panic_mode {
            self.synchronise();
        }
    }

    fn fun_declaration(&mut self) {
        let Some(constant_index) = self.parse_variable("Expect function name.") else {
            return;
        };
        self.mark_initialized();
        self.function(FunctionKind::Function);
        self.define_variable(constant_index);
    }

    fn function(&mut self, function_kind: FunctionKind) {
        self.init_compiler(function_kind);
        self.begin_scope();

        self.consume(TokenKind::LeftParen, "Expect '(' after function name.");
        if !self.check(TokenKind::RightParen) {
            loop {
                let current_arity = &mut self.context.function.arity;
                if *current_arity == FUNCTION_ARITY_MAX_COUNT {
                    self.error_at_current("Can't have more than 255 parameters.");
                    return;
                }
                *current_arity += 1;
                let Some(constant_index) = self.parse_variable("Expect parameter name.") else {
                    return;
                };
                self.define_variable(constant_index);
                if !self.r#match(TokenKind::Comma) {
                    break;
                }
            }
        }

        self.consume(TokenKind::RightParen, "Expect ')' after parameters.");
        self.consume(TokenKind::LeftBrace, "Expect '{' before function body.");
        self.block();

        let Some(function) = self.end_compiler() else {
            self.error("Cannot end compiler for the top-level script.");
            return;
        };
        let Some(constant_index) = self
            .context
            .function
            .chunk
            .add_constant(Value::Function(Rc::new(function)))
        else {
            self.error("Too many constants in one chunk.");
            return;
        };
        self.emit(Opcode::Function as u8);
        self.emit(constant_index);
    }

    fn var_declaration(&mut self) {
        let Some(constant_index) = self.parse_variable("Expect variable name.") else {
            return;
        };
        if self.r#match(TokenKind::Equal) {
            self.expression();
        } else {
            self.emit(Opcode::Nil as u8);
        }
        self.consume(TokenKind::Semicolon, "Expect ';' after value.");
        self.define_variable(constant_index);
    }

    fn parse_variable(&mut self, error_message: &str) -> Option<u8> {
        self.consume(TokenKind::Identifier, error_message);
        self.declare_variable();
        if self.context.scope_depth > 0 {
            // Return dummy variable since locals aren't looked up by name at runtime
            return Some(0);
        }
        self.constant_identifier(self.previous.lexeme)
    }

    fn declare_variable(&mut self) {
        if self.context.scope_depth == 0 {
            return;
        }
        let variable_name = self.previous.lexeme;

        // Detect clashing variable names in current scope (does not include shadowing, which is allowed).
        let mut has_match_name_error = false;
        for local in self.context.locals.iter().rev() {
            if let Some(depth) = local.depth {
                if depth < self.context.scope_depth {
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

    fn add_local(&mut self, name: &'src str) {
        let locals_count = &mut self.context.locals_count;
        if *locals_count == LOCALS_MAX_COUNT {
            self.error("Too many local variables in function.");
            return;
        }
        // "Declare" depth with None, it will later be initialized when variable defined
        self.context.locals[*locals_count].name = name;
        *locals_count += 1;
    }

    fn constant_identifier(&mut self, token_name: &str) -> Option<u8> {
        self.context
            .function
            .chunk
            .add_constant(Value::String(token_name.into()))
    }

    fn define_variable(&mut self, constant_index: u8) {
        if self.context.scope_depth > 0 {
            self.mark_initialized();
            return;
        }
        self.emit(Opcode::DefineGlobal as u8);
        self.emit(constant_index);
    }

    fn mark_initialized(&mut self) {
        if self.context.scope_depth == 0 {
            return;
        }
        self.context.locals[self.context.locals_count - 1].depth = Some(self.context.scope_depth);
    }

    fn statement(&mut self) {
        if self.r#match(TokenKind::Print) {
            self.print_statement();
        } else if self.r#match(TokenKind::For) {
            self.for_statement();
        } else if self.r#match(TokenKind::If) {
            self.if_statement();
        } else if self.r#match(TokenKind::Return) {
            self.return_statement();
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
        self.context.scope_depth += 1;
    }

    fn end_scope(&mut self) {
        self.context.scope_depth -= 1;
        for index in (0..self.context.locals_count).rev() {
            let local = &self.context.locals[index];
            if let Some(depth) = local.depth {
                if depth <= self.context.scope_depth {
                    break;
                }
            }
            self.emit(Opcode::Pop as u8);
            self.context.locals_count -= 1;
            self.context.locals[self.context.locals_count].reset();
        }
    }

    fn block(&mut self) {
        while !self.check(TokenKind::RightBrace) && !self.check(TokenKind::Eof) {
            self.declaration();
        }
        self.consume(TokenKind::RightBrace, "Expect '}' after block.");
    }

    fn print_statement(&mut self) {
        self.expression();
        self.consume(TokenKind::Semicolon, "Expect ';' after value.");
        self.emit(Opcode::Print as u8);
    }

    fn if_statement(&mut self) {
        self.consume(TokenKind::LeftParen, "Expect '(' after 'if'.");
        self.expression();
        self.consume(TokenKind::RightParen, "Expect ')' after condition.");
        let then_jump = self.emit_jump(Opcode::JumpIfFalse as u8);
        self.emit(Opcode::Pop as u8);
        self.statement();
        let else_jump = self.emit_jump(Opcode::Jump as u8);
        self.patch_jump(then_jump);
        self.emit(Opcode::Pop as u8);
        if self.r#match(TokenKind::Else) {
            self.statement();
        }
        self.patch_jump(else_jump);
    }

    fn return_statement(&mut self) {
        let current_function_kind = self.context.function.kind;
        if current_function_kind == FunctionKind::Script {
            self.error("Can't return from top-level code.");
            return;
        }
        if self.r#match(TokenKind::Semicolon) {
            self.emit_return();
        } else {
            self.expression();
            self.consume(TokenKind::Semicolon, "Expect ';' after return value.");
            self.emit(Opcode::Return as u8);
        }
    }

    fn emit_jump(&mut self, operation: u8) -> usize {
        self.emit(operation);
        self.emit(u8::MAX);
        self.emit(u8::MAX);
        self.current_chunk().code.len() - 2
    }

    fn patch_jump(&mut self, index: usize) {
        let jump_offset = self.current_chunk().code.len() - index - 2;
        if jump_offset > u16::MAX as usize {
            self.error("Loop body too large.");
            return;
        }
        let function = &mut self.context.function;
        let bytes = (jump_offset as u16).to_le_bytes();
        function.chunk.code[index] = bytes[0];
        function.chunk.code[index + 1] = bytes[1];
    }

    fn while_statement(&mut self) {
        let loop_start = self.current_chunk().code.len();
        self.consume(TokenKind::LeftParen, "Expect '(' after 'while'.");
        self.expression();
        self.consume(TokenKind::RightParen, "Expect ')' after condition.");

        let exit_jump = self.emit_jump(Opcode::JumpIfFalse as u8);
        self.emit(Opcode::Pop as u8);
        self.statement();
        self.emit_loop(loop_start);

        self.patch_jump(exit_jump);
        self.emit(Opcode::Pop as u8);
    }

    fn for_statement(&mut self) {
        self.begin_scope();
        self.consume(TokenKind::LeftParen, "Expect '(' after 'for'.");
        if self.r#match(TokenKind::Semicolon) {
            // No initializer
        } else if self.r#match(TokenKind::Var) {
            self.var_declaration();
        } else {
            self.expression_statement();
        }
        let mut loop_start = self.current_chunk().code.len();

        // Condition clause.
        let mut exit_jump = None;
        if !self.r#match(TokenKind::Semicolon) {
            self.expression();
            self.consume(TokenKind::Semicolon, "Expect ';' after loop condition.");
            // Jump out of the loop if the condition is false.
            exit_jump = Some(self.emit_jump(Opcode::JumpIfFalse as u8));
            self.emit(Opcode::Pop as u8);
        }

        // Incremenet clause.
        if !self.r#match(TokenKind::RightParen) {
            let body_jump = self.emit_jump(Opcode::Jump as u8);
            let increment_start = self.current_chunk().code.len();
            self.expression();
            self.emit(Opcode::Pop as u8);
            self.consume(TokenKind::RightParen, "Expect ')' after for clauses.");
            self.emit_loop(loop_start);
            loop_start = increment_start;
            self.patch_jump(body_jump);
        }

        self.statement();
        self.emit_loop(loop_start);

        // Patch exit loop jump from condition clause.
        if let Some(exit_jump) = exit_jump {
            self.patch_jump(exit_jump);
            self.emit(Opcode::Pop as u8);
        }
        self.end_scope();
    }

    fn emit_loop(&mut self, loop_start: usize) {
        self.emit(Opcode::Loop as u8);
        let offset = self.current_chunk().code.len() + 2 - loop_start;
        if offset > u16::MAX as usize {
            self.error("Loop body too large.");
            return;
        }
        let bytes = (offset as u16).to_le_bytes();
        self.emit(bytes[0]);
        self.emit(bytes[1]);
    }

    fn expression_statement(&mut self) {
        self.expression();
        self.consume(TokenKind::Semicolon, "Expect ';' after expression.");
        self.emit(Opcode::Pop as u8);
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

    fn emit(&mut self, byte: u8) {
        let line = self.previous.line;
        self.context.function.chunk.write(byte, line);
    }

    fn emit_constant(&mut self, value: Value) {
        let Some(constant_index) = self.context.function.chunk.add_constant(value) else {
            self.error("Too many constants in one chunk.");
            return;
        };
        self.emit(Opcode::Constant as u8);
        self.emit(constant_index);
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

    fn emit_return(&mut self) {
        self.emit(Opcode::Nil as u8);
        self.emit(Opcode::Return as u8);
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

    pub fn parse_precedence(&mut self, precedence: Precedence) {
        self.advance();
        let prefix_rule = Precedence::get_rule(self.previous.kind).prefix;
        if prefix_rule == ParseFunctionKind::None {
            self.error("Expect expression.");
            return;
        };
        self.execute_rule(prefix_rule, precedence);

        while precedence <= Precedence::get_rule(self.current.kind).precedence {
            self.advance();
            let infix_rule = Precedence::get_rule(self.previous.kind).infix;
            self.execute_rule(infix_rule, precedence);
        }

        if Self::can_assign(precedence) && self.r#match(TokenKind::Equal) {
            self.error("Invalid assignment target.");
        }
    }

    fn can_assign(precedence: Precedence) -> bool {
        precedence <= Precedence::Assignment
    }

    fn execute_rule(&mut self, kind: ParseFunctionKind, precedence: Precedence) {
        match kind {
            ParseFunctionKind::None => {}
            ParseFunctionKind::Grouping => Self::grouping(self),
            ParseFunctionKind::Unary => Self::unary(self),
            ParseFunctionKind::Binary => Self::binary(self),
            ParseFunctionKind::Number => Self::number(self),
            ParseFunctionKind::Literal => Self::literal(self),
            ParseFunctionKind::String => Self::string(self),
            ParseFunctionKind::Variable => Self::variable(self, Self::can_assign(precedence)),
            ParseFunctionKind::And => Self::and(self),
            ParseFunctionKind::Or => Self::or(self),
            ParseFunctionKind::Call => Self::call(self),
        }
    }

    pub fn expression(&mut self) {
        self.parse_precedence(Precedence::Assignment);
    }

    fn grouping(&mut self) {
        self.expression();
        self.consume(TokenKind::RightParen, "Expect ')' after expression.");
    }

    fn unary(&mut self) {
        let operator_kind = self.previous.kind;
        self.parse_precedence(Precedence::Unary);
        match operator_kind {
            TokenKind::Minus => self.emit(Opcode::Negate as u8),
            TokenKind::Bang => self.emit(Opcode::Not as u8),
            _ => unreachable!(),
        }
    }

    fn binary(&mut self) {
        let operator_kind = self.previous.kind;
        let parse_rule = Precedence::get_rule(operator_kind);
        self.parse_precedence(parse_rule.precedence.next());
        match operator_kind {
            TokenKind::Plus => self.emit(Opcode::Add as u8),
            TokenKind::Minus => self.emit(Opcode::Subtract as u8),
            TokenKind::Star => self.emit(Opcode::Multiply as u8),
            TokenKind::Slash => self.emit(Opcode::Divide as u8),
            TokenKind::EqualEqual => self.emit(Opcode::Equal as u8),
            TokenKind::Greater => self.emit(Opcode::Greater as u8),
            TokenKind::Less => self.emit(Opcode::Less as u8),
            TokenKind::BangEqual => {
                self.emit(Opcode::Equal as u8);
                self.emit(Opcode::Not as u8);
            }
            TokenKind::GreaterEqual => {
                self.emit(Opcode::Less as u8);
                self.emit(Opcode::Not as u8);
            }
            TokenKind::LessEqual => {
                self.emit(Opcode::Greater as u8);
                self.emit(Opcode::Not as u8);
            }
            _ => unreachable!(),
        }
    }

    fn number(&mut self) {
        let Ok(constant) = self.previous.lexeme.parse() else {
            self.error("Could not cast lexeme to number");
            return;
        };
        self.emit_constant(Value::Number(constant));
    }

    fn literal(&mut self) {
        match self.previous.kind {
            TokenKind::Nil => self.emit(Opcode::Nil as u8),
            TokenKind::True => self.emit(Opcode::True as u8),
            TokenKind::False => self.emit(Opcode::False as u8),
            _ => unreachable!(),
        }
    }

    fn string(&mut self) {
        let lexeme_len = self.previous.lexeme.len();
        let lexeme = &self.previous.lexeme[1..{ lexeme_len - 1 }];
        self.emit_constant(Value::String(lexeme.into()));
    }

    pub fn variable(&mut self, can_assign: bool) {
        self.named_variable(self.previous.lexeme, can_assign);
    }

    pub fn named_variable(&mut self, token_name: &str, can_assign: bool) {
        let (get_operation_bytes, set_operation_bytes) =
            match self.context.resolve_local(token_name) {
                Ok(Some(constant_index)) => (
                    (Opcode::GetLocal as u8, constant_index),
                    (Opcode::SetLocal as u8, constant_index),
                ),
                Ok(None) => {
                    let Some(index) = self.constant_identifier(token_name) else {
                        return;
                    };
                    (
                        (Opcode::GetGlobal as u8, index),
                        (Opcode::SetGlobal as u8, index),
                    )
                }
                Err(e) => {
                    self.error(&e);
                    return;
                }
            };

        if can_assign && self.r#match(TokenKind::Equal) {
            self.expression();
            self.emit(set_operation_bytes.0);
            self.emit(set_operation_bytes.1);
        } else {
            self.emit(get_operation_bytes.0);
            self.emit(get_operation_bytes.1);
        }
    }

    fn and(&mut self) {
        let and_jump = self.emit_jump(Opcode::JumpIfFalse as u8);
        self.emit(Opcode::Pop as u8);
        self.parse_precedence(Precedence::And);
        self.patch_jump(and_jump);
    }

    fn or(&mut self) {
        let else_jump = self.emit_jump(Opcode::JumpIfFalse as u8);
        let end_jump = self.emit_jump(Opcode::Jump as u8);
        self.patch_jump(else_jump);
        self.emit(Opcode::Pop as u8);
        self.parse_precedence(Precedence::Or);
        self.patch_jump(end_jump);
    }

    fn call(&mut self) {
        let Some(argument_count) = self.argument_list() else {
            self.error("Can't have more than 255 arguments.");
            return;
        };
        self.emit(Opcode::Call as u8);
        self.emit(argument_count);
    }

    fn argument_list(&mut self) -> Option<u8> {
        let mut argument_count: u8 = 0;
        if !self.check(TokenKind::RightParen) {
            loop {
                self.expression();
                argument_count = match argument_count.checked_add(1) {
                    Some(count) => count,
                    None => return None,
                };

                if !self.r#match(TokenKind::Comma) {
                    break;
                }
            }
        }
        self.consume(TokenKind::RightParen, "Expect ')' after arguments.");
        Some(argument_count)
    }
}

#[derive(Debug, Default)]
struct Local<'src> {
    pub depth: Option<u8>,
    pub name: &'src str,
}

impl Local<'_> {
    fn reset(&mut self) {
        self.depth = None;
        self.name = "";
    }
}

struct CompilerContext<'src> {
    pub enclosing: Option<Box<CompilerContext<'src>>>,
    function: Function,
    scope_depth: u8,
    locals: [Local<'src>; LOCALS_MAX_COUNT],
    locals_count: usize,
}

impl<'src> CompilerContext<'src> {
    pub fn new(name: &str, function_kind: FunctionKind) -> Self {
        let function = Function::new(name.into(), function_kind);
        let mut locals = std::array::from_fn(|_| Local::default());
        locals[0].depth = Some(0);
        if function_kind != FunctionKind::Function {
            locals[0].name = "this";
        }
        Self {
            enclosing: None,
            function,
            scope_depth: 0,
            locals,
            locals_count: 1,
        }
    }

    fn resolve_local(&self, token_name: &str) -> Result<Option<u8>, String> {
        for (index, local) in self.locals.iter().enumerate().rev() {
            if token_name == local.name {
                if local.depth.is_none() {
                    return Err("Can't read local variable in its own initializer.".into());
                }
                return Ok(Some(index as u8));
            }
        }
        // Assume global variable
        Ok(None)
    }

    #[cfg(feature = "debug_code")]
    fn debug(&self) {
        let name = match self.function.name.is_empty() {
            true => "<script>".into(),
            false => self.function.name.clone(),
        };
        self.function.chunk.disassemble(&name);
    }
}
