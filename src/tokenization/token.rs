/// An enumeration of relations "equal", "not equal",
/// "greater than", "less than", "greater or equal",
/// "less or equal". 
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Relation {
    Eq,
    Ne,
    Gt,
    Lt,
    Ge,
    Le,
}

/// Operators
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Operator {
    Plus,
    Minus,
    Multiply,
    Divide,
    IntegerDivide,
    Modulus,
    And,
    Or,
    Xor,
    Not,
    Assign
}                

/// Keywords
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Keyword {
    If,
    Then,
    Else,
    Of,
    While,
    Do,
    Begin,
    End,
    Var,
    Array,
    Procedure,
    Program,
    Repeat,
    With,
    Until,
    For,
    To,
    Downto,
    Record,
    Type,
}

/// Punctuation symbols
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Punctuation {
    Lbracket,
    Rbracket,
    Lsqbracket,
    Rsqbracket,
    Dot,
    Comma,
    Semicolon,
    Colon,
    Range,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Token {
    O(Operator),
    R(Relation),
    K(Keyword),
    P(Punctuation),
    Literal(String),
    Id(String),
    Number(String),
    EOF,
    Unknown,
}
