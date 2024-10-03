#[derive(Debug, PartialEq, Eq, Clone, Copy, Hash)]
pub enum TokenKind {
    // Single-character tokens.
    LeftParen,
    RightParen,
    LeftBrace,
    RightBrace,
    Comma,
    Dot,
    Minus,
    Plus,
    Semicolon,
    Slash,
    Star,
    // One or two character tokens.
    Bang,
    BangEqual,
    Equal,
    EqualEqual,
    Greater,
    GreaterEqual,
    Less,
    LessEqual,
    // Literals.
    Identifier,
    String,
    Number,
    // Keywords.
    And,    // pairs_with
    Class,  // dish
    Else,   // needs_more_salt
    False,  // bland
    For,    // stir
    Fun,    // recipe
    If,     // taste
    Nil,    // missing_ingredient
    Or,     // alternatively
    Print,  // garnish
    Return, // plate_up
    Super,  // heres_one_i_made_earlier
    This,   // this_dish
    True,   // delicious
    Var,    // ingredient
    While,  // mix_while
    // Other.
    Error,
    Eof,
}

#[derive(Debug, Clone, Copy)]
pub struct Token<'source> {
    pub kind: TokenKind,
    pub lexeme: &'source str,
    pub line: usize,
}

impl<'source> Token<'source> {
    pub fn initial() -> Token<'source> {
        Token {
            kind: TokenKind::Error,
            lexeme: "Found initial token.",
            line: 1,
        }
    }
}
