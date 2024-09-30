#[derive(Debug, PartialEq, Eq, Clone, Copy)]
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
    While,  // mix_until
    // Other.
    Error,
    Eof,
}

#[derive(Debug)]
pub struct Token<'source> {
    pub kind: TokenKind,
    pub lexeme: &'source str,
    pub line: usize,
}
