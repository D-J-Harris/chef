use crate::value::Value;

use crate::{chunk::Operation, scanner::token::TokenKind};

use super::Compiler;

impl Compiler<'_> {
    pub fn parse_precedence(&mut self, precedence: Precedence) {
        self.advance();
        let prefix_rule = Precedence::get_rule(self.previous.kind).prefix;
        if prefix_rule == ParseFunctionKind::None {
            self.error("Expect expression.");
            return;
        };
        self.execute_rule(prefix_rule);

        while precedence <= Precedence::get_rule(self.current.kind).precedence {
            self.advance();
            let infix_rule = Precedence::get_rule(self.previous.kind).infix;
            self.execute_rule(infix_rule);
        }
    }

    fn execute_rule(&mut self, kind: ParseFunctionKind) {
        match kind {
            ParseFunctionKind::None => {}
            ParseFunctionKind::Grouping => Self::grouping(self),
            ParseFunctionKind::Unary => Self::unary(self),
            ParseFunctionKind::Binary => Self::binary(self),
            ParseFunctionKind::Number => Self::number(self),
            ParseFunctionKind::Literal => Self::literal(self),
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
            self.error("Could not case lexeme to number");
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
}

#[derive(PartialEq, Eq, PartialOrd, Ord)]
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
                infix: ParseFunctionKind::None,
                precedence: Precedence::None,
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
                prefix: ParseFunctionKind::None,
                infix: ParseFunctionKind::None,
                precedence: Precedence::None,
            },
            TokenKind::String => ParseRule {
                prefix: ParseFunctionKind::None,
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
                infix: ParseFunctionKind::None,
                precedence: Precedence::None,
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
                infix: ParseFunctionKind::None,
                precedence: Precedence::None,
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
