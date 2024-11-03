use crate::code::Opcode;
use crate::common::{FUNCTION_ARITY_MAX_COUNT, LOCALS_MAX_COUNT};
use crate::native_functions::declare_native_functions;
use crate::rules::{ParseFunctionKind, Precedence};
use crate::scanner::{Token, TokenKind};
use crate::value::{Function, Value};
use crate::{code::Code, scanner::Scanner};

#[derive(PartialEq)]
enum ArgumentPosition {
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
    code: Code,
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
            if let Err(err) = compiler.add_local(name) {
                compiler.error(err);
            }
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
            TokenKind::StepsHeader,
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
        if !self.r#match(TokenKind::IngredientsHeader) {
            return;
        }
        while !self.is_end_ingredients() {
            if !self.check(TokenKind::Var) {
                self.error_at_current("Expect ingredient name.");
                self.synchronise();
                break;
            }
            self.var_declaration();
        }
    }

    fn parse_utensils(&mut self) {
        if !self.r#match(TokenKind::UtensilsHeader) {
            return;
        }
        while !self.is_end_utensils() {
            if !self.check(TokenKind::FunIdent) {
                self.error_at_current("Expect utensil name.");
                self.synchronise();
                break;
            }
            self.fun_declaration();
        }
    }

    fn is_end_ingredients(&self) -> bool {
        self.check(TokenKind::UtensilsHeader)
            || self.check(TokenKind::StepsHeader)
            || self.check(TokenKind::Eof)
    }

    fn is_end_utensils(&self) -> bool {
        self.check(TokenKind::StepsHeader) || self.check(TokenKind::Eof)
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
        self.consume(TokenKind::FunIdent, "Expect utensil identifier name.");
        let name = self.previous.lexeme;
        self.function();
        self.define_variable(name);
    }

    fn function(&mut self) {
        self.begin_compiler();
        self.begin_scope();
        let function_name = self.previous.lexeme;
        let mut function_arity = 0;

        if self.check(TokenKind::With) {
            self.advance();
            let mut order = ArgumentPosition::First;
            loop {
                if function_arity == FUNCTION_ARITY_MAX_COUNT {
                    self.error_at_current("Can't have more than 10 parameters.");
                    return;
                }
                function_arity += 1;
                self.consume(TokenKind::Ident, "Expect parameter name.");
                self.define_variable(self.previous.lexeme);
                match self.current.kind {
                    TokenKind::Comma => {
                        if order == ArgumentPosition::Last {
                            self.error("Invalid ',' after final argument.");
                        }
                        order = ArgumentPosition::Middle;
                        self.advance();
                        continue;
                    }
                    TokenKind::ParameterAnd => {
                        if order == ArgumentPosition::Last {
                            self.error("Invalid 'and' after final argument.");
                        }
                        order = ArgumentPosition::Last;
                        self.advance();
                        continue;
                    }
                    TokenKind::Step => {
                        if order == ArgumentPosition::Middle {
                            self.error("Function parameters should be a list where the final element is preceded by 'and'.");
                        }
                        break;
                    }
                    _ => match order == ArgumentPosition::Middle {
                        true => self.error_at_current("function argument list incomplete"),
                        false => break,
                    },
                }
            }
        }
        let fun_jump = self.emit_jump(Opcode::Jump as u8);
        let function = Function {
            name: function_name.into(),
            arity: function_arity,
            ip_start: self.code.bytes.len(),
        };
        let constant_index = match self.code.add_constant(Value::Function(function)) {
            Ok(constant_index) => constant_index,
            Err(err) => {
                self.error(err);
                return;
            }
        };
        self.block();
        self.end_compiler();
        self.patch_jump(fun_jump);
        self.emit(Opcode::Constant as u8);
        self.emit(constant_index);
    }

    fn var_declaration(&mut self) {
        self.consume(TokenKind::Var, "Expect 'set' ingredient identifier.");
        self.consume(TokenKind::VarIdent, "Expect ingredient identifier name.");
        self.define_variable(self.previous.lexeme);
        if self.r#match(TokenKind::Equal) {
            self.expression();
        } else {
            self.emit(Opcode::Nil as u8);
        }
        if !(self.is_end_ingredients() || self.check(TokenKind::Var)) {
            self.error_at_current("Expect 'set' ingredient identifier.");
        }
    }

    fn define_variable(&mut self, name: &'src str) {
        let mut has_match_name_error = false;
        for local_name in self.context.locals.iter().rev() {
            if *local_name == name {
                has_match_name_error = true
            }
        }
        if has_match_name_error {
            self.error("Already a variable with this name in this scope.");
        }
        if let Err(err) = self.add_local(name) {
            self.error(err);
        }
    }

    pub fn add_local(&mut self, name: &'src str) -> Result<(), &'static str> {
        if self.context.locals_count == LOCALS_MAX_COUNT {
            return Err("Too many locals defined in scope.");
        }
        self.context.locals[self.context.locals_count] = name;
        self.context.locals_count += 1;
        Ok(())
    }

    fn statement(&mut self) {
        if let Some(else_jump) = self.context.active_else {
            match self.r#match(TokenKind::Else) {
                true => {
                    self.else_statement();
                    self.patch_jump(else_jump);
                    self.context.active_else = None;
                    return;
                }
                false => {
                    self.patch_jump(else_jump);
                    self.context.active_else = None;
                }
            };
        }
        if self.check(TokenKind::Step) {
            self.error("Empty instruction.");
        } else if self.r#match(TokenKind::Print) {
            self.print_statement();
        } else if self.r#match(TokenKind::If) {
            self.if_statement();
        } else if self.r#match(TokenKind::Return) {
            self.return_statement();
        } else if self.r#match(TokenKind::While) {
            self.while_statement();
        } else if self.r#match(TokenKind::Else) {
            self.error("'otherwise' clause without a matching 'check' clause.");
        } else {
            self.expression_statement();
        }
    }

    fn begin_scope(&mut self) {
        self.context.scope_ordering.push(1);
    }

    fn end_scope(&mut self) {
        self.context.scope_ordering.pop();
    }

    fn block(&mut self) {
        if !self.r#match(TokenKind::Step) {
            self.end_scope();
            return;
        }
        if self.previous.lexeme != "1.".to_string() {
            self.error("Expect instruction to start from '1.'");
            self.advance();
            return;
        }
        let mut end_found = false;
        loop {
            let current_step = self.context.scope_ordering.last_mut().unwrap();
            if self.previous.lexeme != format!("{current_step}.") {
                self.error("Expect instruction numbers to increase.");
                break;
            }
            match current_step.checked_add(1) {
                Some(n) => *current_step = n,
                None => {
                    self.error("Too many steps.");
                    break;
                }
            };
            if self.r#match(TokenKind::RightBrace) {
                if let Some(else_jump) = self.context.active_else {
                    self.patch_jump(else_jump);
                }
                end_found = true;
                break;
            }
            self.statement();
            if self.panic_mode {
                self.synchronise();
            }
            if !self.r#match(TokenKind::Step) {
                break;
            }
        }
        if !end_found {
            self.error_at_current("Instructions must terminate with 'end'.");
        }
        self.end_scope();
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
        self.begin_scope();
        self.block();
        let else_jump = self.emit_jump(Opcode::Jump as u8);
        self.patch_jump(then_jump);
        self.emit(Opcode::Pop as u8);
        self.context.active_else = Some(else_jump);
    }

    fn else_statement(&mut self) {
        self.begin_scope();
        self.block();
    }

    fn return_statement(&mut self) {
        if self.context.scope_ordering.len() == 1 {
            self.error("Can't return from top-level code.");
        }
        self.expression();
        self.check_end_step();
        self.emit(Opcode::Return as u8);
    }

    fn emit_jump(&mut self, operation: u8) -> usize {
        self.emit(operation);
        self.emit(u8::MAX);
        self.emit(u8::MAX);
        self.code.bytes.len() - 2
    }

    fn patch_jump(&mut self, index: usize) {
        let jump_offset = self.code.bytes.len() - index - 2;
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
        self.begin_scope();
        self.block();
        self.emit_loop(loop_start);

        self.patch_jump(exit_jump);
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
        self.error_at_current(message);
    }

    fn check_end_step(&mut self) {
        if self.current.kind != TokenKind::Step {
            self.error_at_current("Expect next or final instruction in the sequence.")
        }
    }

    fn emit(&mut self, byte: u8) {
        let line = self.previous.line;
        self.code.write(byte, line);
    }

    fn emit_constant(&mut self, value: Value) {
        let constant_index = match self.code.add_constant(value) {
            Ok(constant_index) => constant_index,
            Err(err) => {
                self.error(err);
                return;
            }
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
            TokenKind::Eof => eprint!(" at end of file"),
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
            match self.current.kind {
                TokenKind::If | TokenKind::While | TokenKind::Print | TokenKind::Return => {
                    self.advance();
                    return;
                }
                TokenKind::IngredientsHeader
                | TokenKind::UtensilsHeader
                | TokenKind::StepsHeader
                | TokenKind::Step => return,
                _ => self.advance(),
            }
        }
    }

    pub fn parse_precedence(&mut self, precedence: Precedence) {
        let can_assign = match self.current.kind == TokenKind::Var {
            true => {
                self.advance();
                Self::can_assign(precedence)
            }
            false => false,
        };
        let prefix_rule = Precedence::get_rule(self.current.kind).prefix;
        if prefix_rule == ParseFunctionKind::None {
            match self.current.kind {
                TokenKind::IngredientsHeader
                | TokenKind::UtensilsHeader
                | TokenKind::StepsHeader
                | TokenKind::Eof => self.error_at_current("Expect 'end' step."),
                _ => {
                    self.error_at_current("Expect expression.");
                    self.advance();
                }
            }
            return;
        };
        self.advance();
        self.execute_rule(prefix_rule, can_assign);
        while precedence <= Precedence::get_rule(self.current.kind).precedence {
            let can_assign = self.previous.kind == TokenKind::Var && Self::can_assign(precedence);
            self.advance();
            let infix_rule = Precedence::get_rule(self.previous.kind).infix;
            self.execute_rule(infix_rule, can_assign);
        }

        if Self::can_assign(precedence) && self.r#match(TokenKind::Equal) {
            self.error("Invalid assignment target.");
        }
    }

    fn can_assign(precedence: Precedence) -> bool {
        precedence <= Precedence::Assignment
    }

    fn execute_rule(&mut self, kind: ParseFunctionKind, can_assign: bool) {
        match kind {
            ParseFunctionKind::None => {}
            ParseFunctionKind::Grouping => Self::grouping(self),
            ParseFunctionKind::Unary => Self::unary(self),
            ParseFunctionKind::Binary => Self::binary(self),
            ParseFunctionKind::Number => Self::number(self),
            ParseFunctionKind::Literal => Self::literal(self),
            ParseFunctionKind::String => Self::string(self),
            ParseFunctionKind::Variable => Self::variable(self, can_assign),
            ParseFunctionKind::And => Self::and(self),
            ParseFunctionKind::Or => Self::or(self),
            ParseFunctionKind::Call => Self::call(self),
        }
    }

    pub fn expression(&mut self) {
        self.parse_precedence(Precedence::Assignment);
    }

    fn grouping(&mut self) {
        let is_left_parent = self.previous.kind == TokenKind::LeftParen;
        self.expression();
        if is_left_parent {
            self.consume(
                TokenKind::RightParen,
                "Expect ')' after grouping expression.",
            );
        }
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
        if self.previous.kind == TokenKind::BareFunctionInvocation {
            self.emit(Opcode::Call as u8);
            self.emit(0);
            return;
        }
        let Some(argument_count) = self.argument_list() else {
            self.error("Can't have more than 10 arguments.");
            return;
        };
        self.emit(Opcode::Call as u8);
        self.emit(argument_count);
    }

    fn argument_list(&mut self) -> Option<u8> {
        let mut argument_count: u8 = 0;
        let mut order = ArgumentPosition::First;
        loop {
            self.expression();
            if argument_count == FUNCTION_ARITY_MAX_COUNT {
                return None;
            }
            argument_count += 1;
            match self.current.kind {
                TokenKind::Comma => {
                    if order == ArgumentPosition::Last {
                        self.error_at_current("Invalid ',' after final argument.");
                    }
                    order = ArgumentPosition::Middle;
                    self.advance();
                    continue;
                }
                TokenKind::ParameterAnd => {
                    if order == ArgumentPosition::Last {
                        self.error_at_current("Invalid 'and' after final argument.");
                    }
                    order = ArgumentPosition::Last;
                    self.advance();
                    continue;
                }
                TokenKind::Step => {
                    if order == ArgumentPosition::Middle {
                        self.error_at_current("Function parameters should be a list where the final element is preceded by 'and'.");
                    }
                    break;
                }
                _ => match order == ArgumentPosition::Middle {
                    true => self.error_at_current("function argument list incomplete"),
                    false => break,
                },
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
    scope_ordering: Vec<u16>,
    locals: [&'src str; LOCALS_MAX_COUNT],
    locals_count: usize,
    active_else: Option<usize>,
}

impl<'src> CompilerContext<'src> {
    pub fn new() -> Self {
        Self {
            enclosing: None,
            locals: [""; LOCALS_MAX_COUNT],
            locals_count: 0,
            scope_ordering: vec![1],
            active_else: None,
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
        Err("Undefined variable.")
    }
}
