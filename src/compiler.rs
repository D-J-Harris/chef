use crate::code::Opcode;
use crate::common::{FUNCTION_ARITY_MAX_COUNT, LOCALS_MAX_COUNT};
use crate::native_functions::declare_native_functions;
use crate::rules::{ParseFunctionKind, Precedence};
use crate::scanner::{Token, TokenKind};
use crate::value::{Function, Value};
use crate::{code::Code, scanner::Scanner};

#[derive(PartialEq)]
enum ParamOrder {
    First,
    Middle,
    Last,
}

pub struct Compiler<'src> {
    scanner: Scanner<'src>,
    previous: Token<'src>,
    current: Token<'src>,
    context: CompilerContext<'src>,
    had_error: bool,
    panic_mode: bool,
    pub code: Code,
}

impl<'src> Compiler<'src> {
    pub fn new(source: &'src str) -> Self {
        let initial_token = Token::new("", 1, TokenKind::Error);
        let context = CompilerContext::new();
        let mut compiler = Self {
            scanner: Scanner::new(source),
            previous: initial_token,
            current: initial_token,
            had_error: false,
            panic_mode: false,
            code: Code::new(),
            context,
        };
        for (name, function) in declare_native_functions() {
            compiler.emit_constant(Value::NativeFunction(function));
            compiler.add_local(name);
        }
        compiler
    }

    fn begin_compiler(&mut self) {
        let compiler_context = CompilerContext::new();
        let enclosing_compiler_context = std::mem::replace(&mut self.context, compiler_context);
        self.context.enclosing = Some(Box::new(enclosing_compiler_context));
    }

    fn end_compiler(&mut self) {
        self.emit_return();
        drop(
            self.context
                .enclosing
                .take()
                .map(|parent| std::mem::replace(&mut self.context, *parent)),
        )
    }

    pub fn compile(mut self) -> Option<Code> {
        self.advance();
        self.parse_title();
        self.parse_ingredients();
        self.parse_utensils();
        self.consume(
            TokenKind::Steps,
            "Expect 'Recipe' to contain 'Steps' section",
        );
        self.block();
        self.emit_return();
        #[cfg(feature = "debug_code")]
        self.debug();
        match self.had_error {
            true => None,
            false => Some(self.code),
        }
    }

    fn parse_title(&mut self) {
        if !self.r#match(TokenKind::Recipe) {
            self.error("Script must begin with 'Recipe'.");
        }
    }

    fn parse_ingredients(&mut self) {
        if !self.r#match(TokenKind::Ingredients) {
            return;
        }
        while !self.is_end_ingredients() {
            if !self.check(TokenKind::Var) {
                self.error_at_current("Ingredient declarations should begin with '+'.");
                break;
            }
            self.advance();
            self.var_declaration();
        }
    }

    fn is_end_ingredients(&self) -> bool {
        self.check(TokenKind::Utensils)
            || self.check(TokenKind::Steps)
            || self.check(TokenKind::Eof)
    }

    fn is_end_utensils(&self) -> bool {
        self.check(TokenKind::Steps) || self.check(TokenKind::Eof)
    }

    fn parse_utensils(&mut self) {
        if !self.r#match(TokenKind::Utensils) {
            return;
        }
        while !self.is_end_utensils() {
            if !self.check(TokenKind::Var) {
                self.error_at_current("Utensil declarations should begin with '+'.");
                break;
            }
            self.advance();
            self.fun_declaration();
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

    fn fun_declaration(&mut self) {
        self.consume(TokenKind::FunIdent, "Expect utensil function identifier.");
        self.define_variable();
        self.function();
    }

    fn function(&mut self) {
        self.begin_compiler();
        self.begin_scope();
        let function_name = self.previous.lexeme;
        let mut function_arity = 0;

        if self.check(TokenKind::LeftParen) {
            self.advance();
            let mut order = ParamOrder::First;
            loop {
                if function_arity == FUNCTION_ARITY_MAX_COUNT {
                    self.error_at_current("Can't have more than 255 parameters.");
                    return;
                }
                function_arity += 1;
                self.consume(TokenKind::Ident, "Expect parameter name.");
                self.define_variable();
                match self.current.kind {
                    TokenKind::Comma => {
                        order = ParamOrder::Middle;
                        self.advance();
                        continue;
                    }
                    TokenKind::ParameterAnd => {
                        if order == ParamOrder::Last {
                            self.error(
                            "Can not use the 'and' list keyword multiple times, use ',' instead.",
                        );
                        }
                        order = ParamOrder::Last;
                        self.advance();
                        continue;
                    }
                    TokenKind::Hyphen => {
                        if order == ParamOrder::Middle {
                            self.error("Function parameters should be a list where the final element is preceded by 'and'");
                        }
                        break;
                    }
                    _ => {
                        self.error("Expect ',' or 'and' to continue parameter list, or '-' for the first step.");
                        break;
                    }
                }
            }
        }
        let fun_jump = self.emit_jump(Opcode::Jump as u8);
        let function = Function {
            name: function_name.into(),
            arity: function_arity,
            ip_start: self.code.bytes.len(),
        };
        let Some(constant_index) = self.code.add_constant(Value::Function(function)) else {
            self.error("Too many constants in one chunk.");
            return;
        };
        self.block();
        self.end_compiler();
        self.emit(Opcode::Constant as u8);
        self.emit(constant_index);
        self.patch_jump(fun_jump, 2);
    }

    fn var_declaration(&mut self) {
        self.consume(TokenKind::VarIdent, "Expect ingredient identifier.");
        self.define_variable();
        if self.r#match(TokenKind::Equal) {
            self.expression();
        } else {
            self.emit(Opcode::Nil as u8);
        }
        if !(self.is_end_ingredients() || self.check(TokenKind::Var)) {
            self.error_at_current("Ingredient declaration termination invalid.");
        }
    }

    fn define_variable(&mut self) {
        let name = self.previous.lexeme;
        let mut has_match_name_error = false;
        for local_name in self.context.locals.iter().rev() {
            if *local_name == name {
                has_match_name_error = true
            }
        }
        if has_match_name_error {
            self.error("Already a variable with this name in this scope.");
        }
        self.add_local(name);
    }

    pub fn add_local(&mut self, name: &'src str) {
        let locals_count = &mut self.context.locals_count;
        self.context.locals[*locals_count] = name;
        *locals_count += 1;
    }

    fn statement(&mut self) -> bool {
        if !self.check(TokenKind::Hyphen) {
            self.error_at_current("Steps should end with 'finish' step.");
            return true;
        }
        self.advance();
        if self.r#match(TokenKind::RightBrace) {
            return true;
        } else if self.r#match(TokenKind::Print) {
            self.print_statement();
        } else if self.r#match(TokenKind::If) {
            self.if_statement();
        } else if self.r#match(TokenKind::Return) {
            self.return_statement();
        } else if self.r#match(TokenKind::While) {
            self.while_statement();
        } else {
            self.expression_statement();
        }
        false
    }

    fn begin_scope(&mut self) {
        self.context.scope_depth += 1;
    }

    fn block(&mut self) {
        while self.check(TokenKind::Eof) || self.check(TokenKind::Hyphen) {
            let done = self.statement();
            if self.panic_mode {
                self.synchronise();
            }
            if done {
                break;
            }
        }
    }

    fn print_statement(&mut self) {
        self.expression();
        self.check_end_step();
        self.emit(Opcode::Print as u8);
    }

    fn if_statement(&mut self) {
        self.expression();
        let then_jump = self.emit_jump(Opcode::JumpIfFalse as u8);
        self.emit(Opcode::Pop as u8);
        self.block();
        let else_jump = self.emit_jump(Opcode::Jump as u8);
        self.patch_jump(then_jump, 0);
        self.emit(Opcode::Pop as u8);
        if self.r#match(TokenKind::Else) {
            self.block();
        }
        self.patch_jump(else_jump, 0);
    }

    fn return_statement(&mut self) {
        if self.context.scope_depth == 0 {
            self.error("Can't return from top-level code.");
        }
        if self.r#match(TokenKind::Semicolon) {
            self.emit_return();
        } else {
            self.expression();
            self.check_end_step();
            self.emit(Opcode::Return as u8);
        }
    }

    fn emit_jump(&mut self, operation: u8) -> usize {
        self.emit(operation);
        self.emit(u8::MAX);
        self.emit(u8::MAX);
        self.code.bytes.len() - 2
    }

    fn patch_jump(&mut self, index: usize, offset: usize) {
        let jump_offset = self.code.bytes.len() - index - 2 - offset;
        if jump_offset > u16::MAX as usize {
            self.error("Loop body too large.");
            return;
        }
        let bytes = (jump_offset as u16).to_le_bytes();
        self.code.bytes[index] = bytes[0];
        self.code.bytes[index + 1] = bytes[1];
    }

    fn while_statement(&mut self) {
        let loop_start = self.code.bytes.len();
        self.expression();

        let exit_jump = self.emit_jump(Opcode::JumpIfFalse as u8);
        self.emit(Opcode::Pop as u8);
        self.block();
        self.emit_loop(loop_start);

        self.patch_jump(exit_jump, 0);
        self.emit(Opcode::Pop as u8);
    }

    fn emit_loop(&mut self, loop_start: usize) {
        self.emit(Opcode::Loop as u8);
        let offset = self.code.bytes.len() + 2 - loop_start;
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
        self.check_end_step();
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
        self.error(message);
    }

    fn check_end_step(&mut self) {
        if self.current.kind != TokenKind::Hyphen {
            self.error_at_current("Step termination invalid")
        }
    }

    fn emit(&mut self, byte: u8) {
        let line = self.previous.line;
        self.code.write(byte, line);
    }

    fn emit_constant(&mut self, value: Value) {
        let Some(constant_index) = self.code.add_constant(value) else {
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
            if self.previous.kind == TokenKind::Semicolon || self.current.kind == TokenKind::Hyphen
            {
                return;
            }
            match self.current.kind {
                TokenKind::Var
                | TokenKind::If
                | TokenKind::While
                | TokenKind::Print
                | TokenKind::Recipe
                | TokenKind::Ingredients
                | TokenKind::Utensils
                | TokenKind::Steps
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
            match self.context.resolve_local(token_name, 0) {
                Ok((constant_index, depth)) => (
                    (Opcode::GetLocal as u8, constant_index, depth),
                    (Opcode::SetLocal as u8, constant_index, depth),
                ),
                Err(err) => {
                    self.error(err);
                    return;
                }
            };

        if can_assign && self.r#match(TokenKind::Equal) {
            self.expression();
            self.emit(set_operation_bytes.0);
            self.emit(set_operation_bytes.1);
            self.emit(set_operation_bytes.2);
        } else {
            self.emit(get_operation_bytes.0);
            self.emit(get_operation_bytes.1);
            self.emit(get_operation_bytes.2);
        }
    }

    fn and(&mut self) {
        let and_jump = self.emit_jump(Opcode::JumpIfFalse as u8);
        self.emit(Opcode::Pop as u8);
        self.parse_precedence(Precedence::And);
        self.patch_jump(and_jump, 0);
    }

    fn or(&mut self) {
        let else_jump = self.emit_jump(Opcode::JumpIfFalse as u8);
        let end_jump = self.emit_jump(Opcode::Jump as u8);
        self.patch_jump(else_jump, 0);
        self.emit(Opcode::Pop as u8);
        self.parse_precedence(Precedence::Or);
        self.patch_jump(end_jump, 0);
    }

    fn call(&mut self) {
        if self.previous.kind == TokenKind::BareFunctionInvocation {
            self.emit(Opcode::Call as u8);
            self.emit(0);
            return;
        }
        let Some(argument_count) = self.argument_list() else {
            self.error("Can't have more than 255 arguments.");
            return;
        };
        self.emit(Opcode::Call as u8);
        self.emit(argument_count);
    }

    fn argument_list(&mut self) -> Option<u8> {
        let mut argument_count: u8 = 0;
        let mut order = ParamOrder::First;
        loop {
            self.expression();
            argument_count = match argument_count.checked_add(1) {
                Some(count) => count,
                None => return None,
            };
            match self.current.kind {
                TokenKind::Comma => {
                    if order == ParamOrder::Last {
                        self.error_at_current("Invalid ',' after final argument.");
                    }
                    order = ParamOrder::Middle;
                    self.advance();
                    continue;
                }
                TokenKind::ParameterAnd => {
                    if order == ParamOrder::Last {
                        self.error_at_current("Invalid 'and' after final argument.");
                    }
                    order = ParamOrder::Last;
                    self.advance();
                    continue;
                }
                TokenKind::Hyphen => {
                    if order == ParamOrder::Middle {
                        self.error_at_current("function argument list should terminate with 'and' before final argument.");
                    }
                    break;
                }
                _ => self.error_at_current("Invalid termination of function invocation."),
            }
        }
        Some(argument_count)
    }

    #[cfg(feature = "debug_code")]
    fn debug(&self) {
        self.code.disassemble();
    }
}

struct CompilerContext<'src> {
    enclosing: Option<Box<CompilerContext<'src>>>,
    scope_depth: u8,
    locals: [&'src str; LOCALS_MAX_COUNT],
    locals_count: usize,
}

impl<'src> CompilerContext<'src> {
    pub fn new() -> Self {
        Self {
            enclosing: None,
            locals: [""; LOCALS_MAX_COUNT],
            locals_count: 0,
            scope_depth: 0,
        }
    }

    fn resolve_local(&mut self, token_name: &str, depth: u8) -> Result<(u8, u8), &'static str> {
        for (index, local_name) in self.locals.iter().enumerate().rev() {
            if token_name == *local_name {
                return Ok((index as u8, depth));
            }
        }
        if let Some(parent_compiler) = self.enclosing.as_deref_mut() {
            return parent_compiler.resolve_local(token_name, depth + 1);
        }
        Err("Name not defined in local scope.")
    }
}
