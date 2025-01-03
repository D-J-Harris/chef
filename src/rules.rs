use crate::scanner::TokenKind;

#[derive(PartialEq, Eq, Debug, PartialOrd, Ord, Clone, Copy)]
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

#[derive(Debug, PartialEq, Eq)]
pub enum ParseFunctionKind {
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
    pub prefix: ParseFunctionKind,
    pub infix: ParseFunctionKind,
    pub precedence: Precedence,
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
            TokenKind::With => ParseRule {
                prefix: ParseFunctionKind::Grouping,
                infix: ParseFunctionKind::Call,
                precedence: Precedence::Call,
            },
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
            TokenKind::BareFunctionInvocation => ParseRule {
                prefix: ParseFunctionKind::None,
                infix: ParseFunctionKind::Call,
                precedence: Precedence::Call,
            },
            TokenKind::RightBrace => ParseRule {
                prefix: ParseFunctionKind::None,
                infix: ParseFunctionKind::None,
                precedence: Precedence::None,
            },
            TokenKind::ParameterAnd => ParseRule {
                prefix: ParseFunctionKind::None,
                infix: ParseFunctionKind::None,
                precedence: Precedence::None,
            },
            TokenKind::Comma => ParseRule {
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
            TokenKind::Step => ParseRule {
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
            TokenKind::Less => ParseRule {
                prefix: ParseFunctionKind::None,
                infix: ParseFunctionKind::Binary,
                precedence: Precedence::Comparison,
            },
            TokenKind::VarIdent => ParseRule {
                prefix: ParseFunctionKind::Variable,
                infix: ParseFunctionKind::None,
                precedence: Precedence::None,
            },
            TokenKind::FunIdent => ParseRule {
                prefix: ParseFunctionKind::Variable,
                infix: ParseFunctionKind::None,
                precedence: Precedence::None,
            },
            TokenKind::Ident => ParseRule {
                prefix: ParseFunctionKind::Variable,
                infix: ParseFunctionKind::None,
                precedence: Precedence::None,
            },
            TokenKind::Var => ParseRule {
                prefix: ParseFunctionKind::None,
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
            TokenKind::If => ParseRule {
                prefix: ParseFunctionKind::Grouping,
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
            TokenKind::True => ParseRule {
                prefix: ParseFunctionKind::Literal,
                infix: ParseFunctionKind::None,
                precedence: Precedence::None,
            },
            TokenKind::While => ParseRule {
                prefix: ParseFunctionKind::Grouping,
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
            TokenKind::Recipe => ParseRule {
                prefix: ParseFunctionKind::None,
                infix: ParseFunctionKind::None,
                precedence: Precedence::None,
            },
            TokenKind::IngredientsHeader => ParseRule {
                prefix: ParseFunctionKind::None,
                infix: ParseFunctionKind::None,
                precedence: Precedence::None,
            },
            TokenKind::UtensilsHeader => ParseRule {
                prefix: ParseFunctionKind::None,
                infix: ParseFunctionKind::None,
                precedence: Precedence::None,
            },
            TokenKind::StepsHeader => ParseRule {
                prefix: ParseFunctionKind::None,
                infix: ParseFunctionKind::None,
                precedence: Precedence::None,
            },
        }
    }
}
