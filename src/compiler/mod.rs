use std::rc::Rc;
use std::u8;

use crate::objects::Object;
use crate::objects::{Function, FunctionKind, ObjectString};
use crate::scanner::token::Token;
use crate::{
    chunk::{Chunk, Operation},
    scanner::{token::TokenKind, Scanner},
    value::Value,
};

mod precedence;

pub struct Parser<'source> {
    scanner: Scanner<'source>,
    previous: Token<'source>,
    current: Token<'source>,
    had_error: bool,
    panic_mode: bool,
    compiler: Compiler<'source>,
}

impl<'source> Parser<'source> {
    pub fn new(source: &'source str) -> Self {
        let initial_token = Token::initial();
        Self {
            scanner: Scanner::new(source),
            previous: initial_token,
            current: initial_token,
            had_error: false,
            panic_mode: false,
            compiler: Compiler::new(),
        }
    }

    fn current_chunk(&self) -> &Chunk {
        &self.compiler.context.function.chunk
    }

    fn current_chunk_mut(&mut self) -> &mut Chunk {
        &mut self.compiler.context.function.chunk
    }

    fn current_scope_depth(&self) -> u8 {
        self.compiler.context.scope_depth
    }

    fn current_scope_depth_mut(&mut self) -> &mut u8 {
        &mut self.compiler.context.scope_depth
    }

    pub fn compile(mut self) -> Option<Rc<Function>> {
        self.advance();
        while !self.r#match(TokenKind::Eof) {
            self.declaration();
        }
        let had_error = self.had_error;
        let function = match self.end_compiler() {
            Some(f) => f,
            None => self.compiler.context.function,
        };
        match had_error {
            true => None,
            false => Some(Rc::new(function)),
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
            self.error("Reached constant limit.");
            return;
        };
        self.mark_initialized();
        self.function(FunctionKind::Function);
        self.define_variable(constant_index);
    }

    fn init_compiler(&mut self, function_kind: FunctionKind) {
        let name = self.previous.lexeme; // TODO: heap allocated string interning?
        let compiler_context = CompilerContext::new(name, function_kind);
        let enclosing_compiler_context =
            std::mem::replace(&mut self.compiler.context, compiler_context);
        self.compiler.context.parent = Some(Box::new(enclosing_compiler_context));
    }

    fn function(&mut self, function_kind: FunctionKind) {
        self.init_compiler(function_kind);
        self.begin_scope();

        self.consume(TokenKind::LeftParen, "Expect '(' after function name.");
        if !self.check_current_token(TokenKind::RightParen) {
            loop {
                if self
                    .compiler
                    .context
                    .function
                    .arity
                    .checked_add(1)
                    .is_none()
                {
                    self.error_at_current("Can't have more than 255 parameters.");
                    return;
                }
                let Some(constant_index) = self.parse_variable("Expect parameter name.") else {
                    self.error("Reached constant limit.");
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
            .current_chunk_mut()
            .add_constant(Value::ObjectValue(Object::Function(Rc::new(function))))
        else {
            self.error("Reached constant limit before adding function.");
            return;
        };
        self.emit_operation(Operation::Constant(constant_index));
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
        if self.current_scope_depth() > 0 {
            // Return dummy variable since locals aren't looked up by name at runtime
            return Some(0);
        }
        self.constant_identifier(&self.previous.lexeme)
    }

    fn declare_variable(&mut self) {
        if self.current_scope_depth() == 0 {
            return;
        }
        let variable_name = self.previous.lexeme;

        // Detect clashing variable names in current scope (does not include shadowing, which is allowed).
        let mut has_match_name_error = false;
        for local in self.compiler.context.locals.iter().rev() {
            if let Some(depth) = local.depth {
                if depth < self.current_scope_depth() {
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
        // "Declare" depth with None, it will later be initialized when variable defined
        self.compiler.context.locals[self.compiler.context.locals_count].name = name;
        self.compiler.context.locals_count += 1;
    }

    fn constant_identifier(&mut self, token_name: &str) -> Option<u8> {
        let object_string = Rc::new(ObjectString::new(token_name));
        self.current_chunk_mut()
            .add_constant(Value::ObjectValue(Object::String(object_string)))
    }

    fn define_variable(&mut self, constant_index: u8) {
        if self.current_scope_depth() > 0 {
            self.mark_initialized();
            return;
        }
        self.emit_operation(Operation::DefineGlobal(constant_index));
    }

    fn mark_initialized(&mut self) {
        if self.current_scope_depth() == 0 {
            return;
        }
        self.compiler.context.locals[self.compiler.context.locals_count - 1].depth =
            Some(self.current_scope_depth());
        return;
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
        *self.current_scope_depth_mut() += 1;
    }

    fn end_scope(&mut self) {
        *self.current_scope_depth_mut() -= 1;
        for index in (0..self.compiler.context.locals_count).rev() {
            let local = &self.compiler.context.locals[index];
            if let Some(depth) = local.depth {
                if depth <= self.current_scope_depth() {
                    break;
                }
            }
            self.compiler.context.locals_count -= 1;
            self.compiler.context.locals[self.compiler.context.locals_count].reset();
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
        let num_operations_if = self.current_chunk().code.len();
        self.emit_operation(Operation::Pop);
        self.statement();
        self.emit_operation(Operation::Jump(u8::MAX));
        let num_operations_else = self.current_chunk().code.len();
        self.patch_jump(num_operations_if);
        self.emit_operation(Operation::Pop);
        if self.r#match(TokenKind::Else) {
            self.statement();
        }
        self.patch_jump(num_operations_else);
    }

    fn patch_jump(&mut self, num_operations_before: usize) {
        let num_operations_after = self.current_chunk().code.len();
        if num_operations_after - num_operations_before > u8::MAX as usize {
            self.error("Too much code to jump over.");
            return;
        }
        match self
            .current_chunk_mut()
            .code
            .get_mut(num_operations_before - 1)
        {
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
        let num_operations_loop_start = self.current_chunk().code.len();
        self.consume(TokenKind::LeftParen, "Expect '(' after 'while'.");
        self.expression();
        self.consume(TokenKind::RightParen, "Expect ')' after condition.");

        self.emit_operation(Operation::JumpIfFalse(u8::MAX));
        let num_operations_exit = self.current_chunk().code.len();
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
            // No initializer
        } else if self.r#match(TokenKind::Var) {
            self.var_declaration();
        } else {
            self.expression_statement();
        }
        let mut num_operations_loop_start = self.current_chunk().code.len();

        // Condition clause.
        let mut num_operations_exit = None;
        if !self.r#match(TokenKind::Semicolon) {
            self.expression();
            self.consume(TokenKind::Semicolon, "Expect ';' after loop condition.");
            // Jump out of the loop if the condition is false.
            self.emit_operation(Operation::JumpIfFalse(u8::MAX));
            num_operations_exit = Some(self.current_chunk().code.len());
            self.emit_operation(Operation::Pop);
        }

        // Incremenet clause.
        if !self.r#match(TokenKind::RightParen) {
            self.emit_operation(Operation::Jump(u8::MAX));
            let num_operations_jump = self.current_chunk().code.len();
            let num_operations_increment_start = self.current_chunk().code.len();
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
        let offset = self.current_chunk().code.len() - num_operations_loop_start;
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
        let line = self.previous.line;
        self.current_chunk_mut().write(operation, line);
    }

    fn emit_constant(&mut self, constant: Value) {
        if let Some(constant_index) = self.current_chunk_mut().add_constant(constant) {
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

    fn end_compiler(&mut self) -> Option<Function> {
        self.emit_operation(Operation::Nil);
        self.emit_operation(Operation::Return);
        #[cfg(feature = "debug_print_code")]
        self.compiler.debug();

        self.compiler.context.parent.take().map(|parent| {
            let context: CompilerContext<'source> =
                std::mem::replace(&mut self.compiler.context, *parent);
            context.function
        })
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

#[derive(Debug, Default)]
struct Local<'source> {
    depth: Option<u8>,
    name: &'source str,
}

impl Local<'_> {
    fn reset(&mut self) {
        self.depth = None;
        self.name = "";
    }
}

struct CompilerContext<'source> {
    parent: Option<Box<CompilerContext<'source>>>,
    function: Function,
    scope_depth: u8,
    locals: [Local<'source>; u8::MAX as usize],
    locals_count: usize,
}

impl<'source> CompilerContext<'source> {
    pub fn new(name: &str, function_kind: FunctionKind) -> Self {
        let function = Function::new(name, function_kind);
        let mut locals = std::array::from_fn(|_| Local::default());
        locals[0].depth = Some(0);
        Self {
            parent: None,
            function,
            scope_depth: 0,
            locals,
            locals_count: 1,
        }
    }
}

pub struct Compiler<'source> {
    context: CompilerContext<'source>,
}

impl<'source> Compiler<'source> {
    pub fn new() -> Self {
        let compiler_context = CompilerContext::new("", FunctionKind::Script);
        Self {
            context: compiler_context,
        }
    }

    #[cfg(feature = "debug_print_code")]
    fn debug(&self) {
        let name = match self.context.function.name.is_empty() {
            true => "<script>".into(),
            false => self.context.function.name.clone(),
        };
        self.context.function.chunk.disassemble(&name);
    }
}
