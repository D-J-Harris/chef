use std::borrow::{Borrow, BorrowMut};
use std::rc::Rc;
use std::u8;

use crate::objects::ObjectString;
use crate::value::Value;

use crate::objects::Object;
use crate::{chunk::Operation, scanner::token::TokenKind};

use super::{Compiler, CompilerContext, Parser};

impl Parser<'_> {
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
            TokenKind::Minus => self.emit_operation(Operation::Negate),
            TokenKind::Bang => self.emit_operation(Operation::Not),
            _ => unreachable!(),
        }
    }

    fn binary(&mut self) {
        let operator_kind = self.previous.kind;
        let parse_rule = Precedence::get_rule(operator_kind);
        self.parse_precedence(parse_rule.precedence.next());
        match operator_kind {
            TokenKind::Plus => self.emit_operation(Operation::Add),
            TokenKind::Minus => self.emit_operation(Operation::Subtract),
            TokenKind::Star => self.emit_operation(Operation::Multiply),
            TokenKind::Slash => self.emit_operation(Operation::Divide),
            TokenKind::EqualEqual => self.emit_operation(Operation::Equal),
            TokenKind::Greater => self.emit_operation(Operation::Greater),
            TokenKind::Less => self.emit_operation(Operation::Less),
            TokenKind::BangEqual => {
                self.emit_operation(Operation::Equal);
                self.emit_operation(Operation::Not);
            }
            TokenKind::GreaterEqual => {
                self.emit_operation(Operation::Less);
                self.emit_operation(Operation::Not);
            }
            TokenKind::LessEqual => {
                self.emit_operation(Operation::Greater);
                self.emit_operation(Operation::Not);
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
            TokenKind::Nil => self.emit_operation(Operation::Nil),
            TokenKind::True => self.emit_operation(Operation::True),
            TokenKind::False => self.emit_operation(Operation::False),
            _ => unreachable!(),
        }
    }

    fn string(&mut self) {
        let lexeme_len = self.previous.lexeme.len();
        let constant = &self.previous.lexeme[1..{ lexeme_len - 1 }];
        let object_string = Rc::new(ObjectString::new(constant));
        self.emit_constant(Value::ObjectValue(Object::String(object_string)));
    }

    fn variable(&mut self, can_assign: bool) {
        self.named_variable(self.previous.lexeme, can_assign);
    }

    fn named_variable(&mut self, token_name: &str, can_assign: bool) {
        let (get_operation, set_operation) = match resolve_local(&self.compiler.context, token_name)
        {
            Ok(Some(constant_index)) => (
                Operation::GetLocal(constant_index),
                Operation::SetLocal(constant_index),
            ),
            Ok(None) => match resolve_upvalue(&mut self.compiler.context, token_name) {
                Ok(Some(upvalue_index)) => (
                    Operation::GetUpvalue(upvalue_index),
                    Operation::SetUpvalue(upvalue_index),
                ),
                Ok(None) => {
                    let Some(index) = self.constant_identifier(token_name) else {
                        self.error("Reached constant limit.");
                        return;
                    };
                    (Operation::GetGlobal(index), Operation::SetGlobal(index))
                }
                Err(e) => {
                    self.error(&e);
                    return;
                }
            },
            Err(e) => {
                self.error(&e);
                return;
            }
        };

        if can_assign && self.r#match(TokenKind::Equal) {
            self.expression();
            self.emit_operation(set_operation);
        } else {
            self.emit_operation(get_operation);
        }
    }

    fn and(&mut self) {
        self.emit_operation(Operation::JumpIfFalse(u8::MAX));
        let operations_before_and = self.current_chunk().code.len();
        self.emit_operation(Operation::Pop);
        self.parse_precedence(Precedence::And);
        self.patch_jump(operations_before_and);
    }

    fn or(&mut self) {
        self.emit_operation(Operation::JumpIfFalse(u8::MAX));
        let operations_before_else_jump = self.current_chunk().code.len();
        self.emit_operation(Operation::Jump(u8::MAX));
        let operations_before_end_jump = self.current_chunk().code.len();

        self.patch_jump(operations_before_else_jump);
        self.emit_operation(Operation::Pop);
        self.parse_precedence(Precedence::Or);
        self.patch_jump(operations_before_end_jump);
    }

    fn call(&mut self) {
        let Some(argument_count) = self.argument_list() else {
            self.error("Can't have more than 255 arguments.");
            return;
        };
        self.emit_operation(Operation::Call(argument_count));
    }

    fn argument_list(&mut self) -> Option<u8> {
        let mut argument_count: u8 = 0;
        if !self.check_current_token(TokenKind::RightParen) {
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

fn resolve_local(compiler: &CompilerContext, token_name: &str) -> Result<Option<u8>, String> {
    for (index, local) in compiler.locals.iter().enumerate().rev() {
        if token_name == local.name {
            if local.depth.is_none() {
                return Err("Can't read local variable in its own initializer.".into());
            }
            return Ok(Some(index as u8));
        }
    }
    // Assume global variable
    return Ok(None);
}

fn resolve_upvalue(compiler: &mut CompilerContext, token_name: &str) -> Result<Option<u8>, String> {
    if let Some(parent_compiler) = compiler.parent.as_deref_mut() {
        if let Some(local_index) = resolve_local(parent_compiler, token_name)? {
            return add_upvalue(compiler, local_index, true);
        }
        if let Some(upvalue_index) = resolve_upvalue(parent_compiler, token_name)? {
            return add_upvalue(compiler, upvalue_index, false);
        }
    }
    Ok(None)
}

fn add_upvalue(
    compiler: &mut CompilerContext,
    index: u8,
    is_local: bool,
) -> Result<Option<u8>, String> {
    let upvalue_count = &mut compiler.function.upvalue_count;
    for i in 0..*upvalue_count {
        let upvalue = &mut compiler.upvalues[i as usize];
        if upvalue.index == index && upvalue.is_local == is_local {
            return Ok(Some(i));
        }
    }
    if *upvalue_count == u8::MAX {
        return Err("Too many closure variables in function.".into());
    }
    compiler.upvalues[*upvalue_count as usize].is_local = is_local;
    compiler.upvalues[*upvalue_count as usize].index = index;
    *upvalue_count += 1;
    Ok(Some(*upvalue_count - 1))
}

#[derive(PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
pub enum Precedence {
    None,
    Assignment, // =
    Or,         // or
    And,        // and
    Equality,   // == !=
    Comparison, // < > <= >=
    Term,       // + -
    Factor,     // * /
    Unary,      // ! -
    Call,       // . ()
    Primary,
}

#[derive(PartialEq, Eq)]
enum ParseFunctionKind {
    None,
    Grouping,
    Unary,
    Binary,
    Number,
    Literal,
    String,
    Variable,
    And,
    Or,
    Call,
}

pub struct ParseRule {
    prefix: ParseFunctionKind,
    infix: ParseFunctionKind,
    precedence: Precedence,
}

impl Precedence {
    pub fn next(&self) -> Precedence {
        match self {
            Precedence::None => Precedence::Assignment,
            Precedence::Assignment => Precedence::Or,
            Precedence::Or => Precedence::And,
            Precedence::And => Precedence::Equality,
            Precedence::Equality => Precedence::Comparison,
            Precedence::Comparison => Precedence::Term,
            Precedence::Term => Precedence::Factor,
            Precedence::Factor => Precedence::Unary,
            Precedence::Unary => Precedence::Call,
            Precedence::Call => Precedence::Primary,
            Precedence::Primary => unreachable!(),
        }
    }

    pub fn get_rule(token_kind: TokenKind) -> ParseRule {
        match token_kind {
            TokenKind::LeftParen => ParseRule {
                prefix: ParseFunctionKind::Grouping,
                infix: ParseFunctionKind::Call,
                precedence: Precedence::Call,
            },
            TokenKind::RightParen => ParseRule {
                prefix: ParseFunctionKind::None,
                infix: ParseFunctionKind::None,
                precedence: Precedence::None,
            },
            TokenKind::LeftBrace => ParseRule {
                prefix: ParseFunctionKind::None,
                infix: ParseFunctionKind::None,
                precedence: Precedence::None,
            },
            TokenKind::RightBrace => ParseRule {
                prefix: ParseFunctionKind::None,
                infix: ParseFunctionKind::None,
                precedence: Precedence::None,
            },
            TokenKind::Comma => ParseRule {
                prefix: ParseFunctionKind::None,
                infix: ParseFunctionKind::None,
                precedence: Precedence::None,
            },
            TokenKind::Dot => ParseRule {
                prefix: ParseFunctionKind::None,
                infix: ParseFunctionKind::None,
                precedence: Precedence::None,
            },
            TokenKind::Minus => ParseRule {
                prefix: ParseFunctionKind::Unary,
                infix: ParseFunctionKind::Binary,
                precedence: Precedence::Term,
            },
            TokenKind::Plus => ParseRule {
                prefix: ParseFunctionKind::None,
                infix: ParseFunctionKind::Binary,
                precedence: Precedence::Term,
            },
            TokenKind::Semicolon => ParseRule {
                prefix: ParseFunctionKind::None,
                infix: ParseFunctionKind::None,
                precedence: Precedence::None,
            },
            TokenKind::Slash => ParseRule {
                prefix: ParseFunctionKind::None,
                infix: ParseFunctionKind::Binary,
                precedence: Precedence::Factor,
            },
            TokenKind::Star => ParseRule {
                prefix: ParseFunctionKind::None,
                infix: ParseFunctionKind::Binary,
                precedence: Precedence::Factor,
            },
            TokenKind::Bang => ParseRule {
                prefix: ParseFunctionKind::Unary,
                infix: ParseFunctionKind::None,
                precedence: Precedence::None,
            },
            TokenKind::BangEqual => ParseRule {
                prefix: ParseFunctionKind::None,
                infix: ParseFunctionKind::Binary,
                precedence: Precedence::Equality,
            },
            TokenKind::Equal => ParseRule {
                prefix: ParseFunctionKind::None,
                infix: ParseFunctionKind::None,
                precedence: Precedence::None,
            },
            TokenKind::EqualEqual => ParseRule {
                prefix: ParseFunctionKind::None,
                infix: ParseFunctionKind::Binary,
                precedence: Precedence::Equality,
            },
            TokenKind::Greater => ParseRule {
                prefix: ParseFunctionKind::None,
                infix: ParseFunctionKind::Binary,
                precedence: Precedence::Comparison,
            },
            TokenKind::GreaterEqual => ParseRule {
                prefix: ParseFunctionKind::None,
                infix: ParseFunctionKind::Binary,
                precedence: Precedence::Comparison,
            },
            TokenKind::Less => ParseRule {
                prefix: ParseFunctionKind::None,
                infix: ParseFunctionKind::Binary,
                precedence: Precedence::Comparison,
            },
            TokenKind::LessEqual => ParseRule {
                prefix: ParseFunctionKind::None,
                infix: ParseFunctionKind::Binary,
                precedence: Precedence::Comparison,
            },
            TokenKind::Identifier => ParseRule {
                prefix: ParseFunctionKind::Variable,
                infix: ParseFunctionKind::None,
                precedence: Precedence::None,
            },
            TokenKind::String => ParseRule {
                prefix: ParseFunctionKind::String,
                infix: ParseFunctionKind::None,
                precedence: Precedence::None,
            },
            TokenKind::Number => ParseRule {
                prefix: ParseFunctionKind::Number,
                infix: ParseFunctionKind::None,
                precedence: Precedence::None,
            },
            TokenKind::And => ParseRule {
                prefix: ParseFunctionKind::None,
                infix: ParseFunctionKind::And,
                precedence: Precedence::And,
            },
            TokenKind::Class => ParseRule {
                prefix: ParseFunctionKind::None,
                infix: ParseFunctionKind::None,
                precedence: Precedence::None,
            },
            TokenKind::Else => ParseRule {
                prefix: ParseFunctionKind::None,
                infix: ParseFunctionKind::None,
                precedence: Precedence::None,
            },
            TokenKind::False => ParseRule {
                prefix: ParseFunctionKind::Literal,
                infix: ParseFunctionKind::None,
                precedence: Precedence::None,
            },
            TokenKind::For => ParseRule {
                prefix: ParseFunctionKind::None,
                infix: ParseFunctionKind::None,
                precedence: Precedence::None,
            },
            TokenKind::Fun => ParseRule {
                prefix: ParseFunctionKind::None,
                infix: ParseFunctionKind::None,
                precedence: Precedence::None,
            },
            TokenKind::If => ParseRule {
                prefix: ParseFunctionKind::None,
                infix: ParseFunctionKind::None,
                precedence: Precedence::None,
            },
            TokenKind::Nil => ParseRule {
                prefix: ParseFunctionKind::Literal,
                infix: ParseFunctionKind::None,
                precedence: Precedence::None,
            },
            TokenKind::Or => ParseRule {
                prefix: ParseFunctionKind::None,
                infix: ParseFunctionKind::Or,
                precedence: Precedence::Or,
            },
            TokenKind::Print => ParseRule {
                prefix: ParseFunctionKind::None,
                infix: ParseFunctionKind::None,
                precedence: Precedence::None,
            },
            TokenKind::Return => ParseRule {
                prefix: ParseFunctionKind::None,
                infix: ParseFunctionKind::None,
                precedence: Precedence::None,
            },
            TokenKind::Super => ParseRule {
                prefix: ParseFunctionKind::None,
                infix: ParseFunctionKind::None,
                precedence: Precedence::None,
            },
            TokenKind::This => ParseRule {
                prefix: ParseFunctionKind::None,
                infix: ParseFunctionKind::None,
                precedence: Precedence::None,
            },
            TokenKind::True => ParseRule {
                prefix: ParseFunctionKind::Literal,
                infix: ParseFunctionKind::None,
                precedence: Precedence::None,
            },
            TokenKind::Var => ParseRule {
                prefix: ParseFunctionKind::None,
                infix: ParseFunctionKind::None,
                precedence: Precedence::None,
            },
            TokenKind::While => ParseRule {
                prefix: ParseFunctionKind::None,
                infix: ParseFunctionKind::None,
                precedence: Precedence::None,
            },
            TokenKind::Error => ParseRule {
                prefix: ParseFunctionKind::None,
                infix: ParseFunctionKind::None,
                precedence: Precedence::None,
            },
            TokenKind::Eof => ParseRule {
                prefix: ParseFunctionKind::None,
                infix: ParseFunctionKind::None,
                precedence: Precedence::None,
            },
        }
    }
}
