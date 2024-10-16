use std::rc::Rc;
use std::u8;

use crate::common::{
    FUNCTION_ARITY_MAX_COUNT, JUMP_MAX_COUNT, JUMP_MAX_COUNT_USIZE, LOCALS_MAX_COUNT, SUPER_STRING,
    UPVALUES_MAX_COUNT,
};
use crate::objects::{FunctionKind, FunctionObject};
use crate::scanner::{Token, TokenKind};
use crate::value::Value;
use crate::{
    chunk::{Chunk, Operation},
    scanner::Scanner,
};

mod precedence;

pub struct Compiler<'source> {
    scanner: Scanner<'source>,
    previous: Token<'source>,
    current: Token<'source>,
    context: CompilerContext<'source>,
    class_compiler: Option<Box<ClassCompiler>>,
    had_error: bool,
    panic_mode: bool,
}

impl<'source> Compiler<'source> {
    pub fn new(source: &'source str) -> Self {
        let initial_token = Token::new("", 1, TokenKind::Error);
        let context = CompilerContext::new("", FunctionKind::Script);
        Self {
            scanner: Scanner::new(source),
            previous: initial_token,
            current: initial_token,
            had_error: false,
            panic_mode: false,
            context,
            class_compiler: None,
        }
    }

    fn current_chunk(&self) -> &Chunk {
        &self.context.function.chunk
    }

    fn current_chunk_mut(&mut self) -> &mut Chunk {
        &mut self.context.function.chunk
    }

    pub fn compile(mut self) -> Option<FunctionObject> {
        self.advance();
        while !self.r#match(TokenKind::Eof) {
            self.declaration();
        }
        let had_error = self.had_error;
        let function = match self.end_compiler() {
            Some((f, _upvalues)) => f,
            None => self.context.function,
        };
        match had_error {
            true => None,
            false => Some(function),
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
        if self.r#match(TokenKind::Class) {
            self.class_declaration();
        } else if self.r#match(TokenKind::Fun) {
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

    fn class_declaration(&mut self) {
        self.consume(TokenKind::Identifier, "Expect class name.");
        let class_name = self.previous.lexeme;
        let Some(name_constant_index) = self.constant_identifier(class_name) else {
            self.error("No constants defined.");
            return;
        };
        self.declare_variable();
        self.emit(Operation::Class(name_constant_index));
        self.define_variable(name_constant_index);

        let current_class_compiler = self.class_compiler.take();
        let class_compiler = Box::new(ClassCompiler::new(current_class_compiler));
        self.class_compiler = Some(class_compiler);

        if self.r#match(TokenKind::Less) {
            self.consume(TokenKind::Identifier, "Expect superclass name.");
            self.variable(false);
            if class_name == self.previous.lexeme {
                self.error("A class can't inherit from itself.");
            }

            self.begin_scope();
            self.add_local(SUPER_STRING);
            self.define_variable(0);
            self.named_variable(class_name, false);
            self.emit(Operation::Inherit);
            self.class_compiler.as_mut().unwrap().has_superclass = true;
        }

        self.named_variable(class_name, false);
        self.consume(TokenKind::LeftBrace, "Expect '{' before class body.");
        while !self.check(TokenKind::RightBrace) && !self.check(TokenKind::Eof) {
            self.method();
        }
        self.consume(TokenKind::RightBrace, "Expect '}' after class body.");
        self.emit(Operation::Pop);
        let current_class_compiler = self.class_compiler.take().unwrap();
        if current_class_compiler.has_superclass {
            self.end_scope();
        }
        self.class_compiler = current_class_compiler.enclosing;
    }

    fn method(&mut self) {
        self.consume(TokenKind::Identifier, "Expect method name.");
        let Some(constant_index) = self.constant_identifier(self.previous.lexeme) else {
            self.error("No constants defined.");
            return;
        };
        let function_kind = match self.previous.lexeme {
            "init" => FunctionKind::Initializer,
            _ => FunctionKind::Method,
        };
        self.function(function_kind);
        self.emit(Operation::Method(constant_index));
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
        let name = self.previous.lexeme;
        let compiler_context = CompilerContext::new(name, function_kind);
        let enclosing_compiler_context = std::mem::replace(&mut self.context, compiler_context);
        self.context.enclosing = Some(Box::new(enclosing_compiler_context));
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

        let Some((function, upvalues)) = self.end_compiler() else {
            self.error("Cannot end compiler for the top-level script.");
            return;
        };
        let upvalue_count = function.upvalue_count;

        let Some(constant_index) = self
            .current_chunk_mut()
            .add_constant(Value::Function(Rc::new(function)))
        else {
            self.error("Too many constants in one chunk.");
            return;
        };
        self.emit(Operation::Closure(constant_index));

        // emit bytes for variable number of closure upvalues
        for upvalue in upvalues.iter().take(upvalue_count) {
            self.emit(Operation::ClosureIsLocalByte(upvalue.is_local));
            self.emit(Operation::ClosureIndexByte(upvalue.index));
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
            self.emit(Operation::Nil);
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

    fn add_local(&mut self, name: &'source str) {
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
        self.current_chunk_mut()
            .add_constant(Value::String(token_name.into()))
    }

    fn define_variable(&mut self, constant_index: u8) {
        if self.context.scope_depth > 0 {
            self.mark_initialized();
            return;
        }
        self.emit(Operation::DefineGlobal(constant_index));
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
            if self.context.locals[self.context.locals_count - 1].is_captured {
                self.emit(Operation::CloseUpvalue);
            } else {
                self.emit(Operation::Pop);
            }
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
        self.emit(Operation::Print);
    }

    fn if_statement(&mut self) {
        self.consume(TokenKind::LeftParen, "Expect '(' after 'if'.");
        self.expression();
        self.consume(TokenKind::RightParen, "Expect ')' after condition.");

        self.emit(Operation::JumpIfFalse(JUMP_MAX_COUNT));
        let num_operations_if = self.current_chunk().code.len();
        self.emit(Operation::Pop);
        self.statement();
        self.emit(Operation::Jump(JUMP_MAX_COUNT));
        let num_operations_else = self.current_chunk().code.len();
        self.patch_jump(num_operations_if);
        self.emit(Operation::Pop);
        if self.r#match(TokenKind::Else) {
            self.statement();
        }
        self.patch_jump(num_operations_else);
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
            if current_function_kind == FunctionKind::Initializer {
                self.error("Can't return a value from an initializer.");
            }
            self.expression();
            self.consume(TokenKind::Semicolon, "Expect ';' after return value.");
            self.emit(Operation::Return);
        }
    }

    fn patch_jump(&mut self, num_operations_before: usize) {
        let num_operations_after = self.current_chunk().code.len();
        if num_operations_after - num_operations_before > JUMP_MAX_COUNT_USIZE {
            self.error("Loop body too large.");
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
            }
        }
    }

    fn while_statement(&mut self) {
        let num_operations_loop_start = self.current_chunk().code.len();
        self.consume(TokenKind::LeftParen, "Expect '(' after 'while'.");
        self.expression();
        self.consume(TokenKind::RightParen, "Expect ')' after condition.");

        self.emit(Operation::JumpIfFalse(JUMP_MAX_COUNT));
        let num_operations_exit = self.current_chunk().code.len();
        self.emit(Operation::Pop);
        self.statement();
        self.emit_loop(num_operations_loop_start);

        self.patch_jump(num_operations_exit);
        self.emit(Operation::Pop);
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
            self.emit(Operation::JumpIfFalse(JUMP_MAX_COUNT));
            num_operations_exit = Some(self.current_chunk().code.len());
            self.emit(Operation::Pop);
        }

        // Incremenet clause.
        if !self.r#match(TokenKind::RightParen) {
            self.emit(Operation::Jump(JUMP_MAX_COUNT));
            let num_operations_jump = self.current_chunk().code.len();
            let num_operations_increment_start = self.current_chunk().code.len();
            self.expression();
            self.emit(Operation::Pop);
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
            self.emit(Operation::Pop);
        }
        self.end_scope();
    }

    fn emit_loop(&mut self, num_operations_loop_start: usize) {
        let offset = self.current_chunk().code.len() - num_operations_loop_start;
        if offset > JUMP_MAX_COUNT_USIZE {
            self.error("Loop body too large.");
            return;
        }
        self.emit(Operation::Loop(offset as u8));
    }

    fn expression_statement(&mut self) {
        self.expression();
        self.consume(TokenKind::Semicolon, "Expect ';' after expression.");
        self.emit(Operation::Pop);
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

    fn emit(&mut self, operation: Operation) {
        let line = self.previous.line;
        self.current_chunk_mut().write(operation, line);
    }

    fn emit_constant(&mut self, value: Value) {
        if let Some(constant_index) = self.current_chunk_mut().add_constant(value) {
            self.emit(Operation::Constant(constant_index));
        } else {
            self.error("Too many constants in one chunk.");
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

    fn end_compiler(&mut self) -> Option<(FunctionObject, [Upvalue; UPVALUES_MAX_COUNT])> {
        self.emit_return();
        #[cfg(feature = "debug_trace")]
        self.compiler.debug();

        self.context.enclosing.take().map(|parent| {
            let context: CompilerContext<'source> = std::mem::replace(&mut self.context, *parent);
            (context.function, context.upvalues)
        })
    }

    fn emit_return(&mut self) {
        match self.context.function.kind {
            FunctionKind::Initializer => self.emit(Operation::GetLocal(0)),
            _ => self.emit(Operation::Nil),
        };
        self.emit(Operation::Return);
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
    pub depth: Option<u8>,
    pub name: &'source str,
    pub is_captured: bool, // defaults to false
}

impl Local<'_> {
    fn reset(&mut self) {
        self.depth = None;
        self.name = "";
        self.is_captured = false;
    }
}

#[derive(Default, Debug)]
struct Upvalue {
    pub is_local: bool,
    pub index: u8,
}

struct CompilerContext<'source> {
    pub enclosing: Option<Box<CompilerContext<'source>>>,
    function: FunctionObject,
    scope_depth: u8,
    locals: [Local<'source>; LOCALS_MAX_COUNT],
    locals_count: usize,
    pub upvalues: [Upvalue; UPVALUES_MAX_COUNT],
}

impl<'source> CompilerContext<'source> {
    pub fn new(name: &str, function_kind: FunctionKind) -> Self {
        let function = FunctionObject::new(name.into(), function_kind);
        let upvalues = std::array::from_fn(|_| Upvalue::default());
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
            upvalues,
        }
    }

    #[cfg(feature = "debug_trace")]
    fn debug(&self) {
        let name = match self.function.name.is_empty() {
            true => "<script>".into(),
            false => self.function.name.clone(),
        };
        self.context.function.chunk.disassemble(&name);
    }
}

pub struct ClassCompiler {
    enclosing: Option<Box<ClassCompiler>>,
    has_superclass: bool,
}

impl ClassCompiler {
    fn new(enclosing: Option<Box<ClassCompiler>>) -> Self {
        Self {
            enclosing,
            has_superclass: false,
        }
    }
}
