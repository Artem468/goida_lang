#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    // Ключевые слова
    Let,      // let
    If,       // if
    Else,     // else
    While,    // while
    For,      // for
    Function, // function
    Return,   // return
    Number,   // int/number type
    Text,     // string type
    Boolean,  // bool type
    True,     // true
    False,    // false
    Print,    // print
    Input,    // input
    Import,   // import

    // Литералы
    NumericalLiteral(i64),
    TextLiteral(String),
    Identifier(String),

    // Операторы
    Plus,      // +
    Minus,     // -
    Multiply,  // *
    Divide,    // /
    Remainder, // %
    Equal,     // ==
    Unequal,   // !=
    More,      // >
    Less,      // <
    MoreEqual, // >=
    LessEqual, // <=
    And,       // &&
    Or,        // ||
    Not,       // !
    Assign,    // =

    // Пунктуация
    LeftParentheses,  // (
    RightParentheses, // )
    LeftBrace,        // {
    RightBrace,       // }
    LeftBracket,      // [
    RightBracket,     // ]
    SemicolonPoint,   // ;
    Comma,            // ,
    Point,            // .
    Colon,            // :

    // Специальные
    EndFile,
    NewLine,
}

pub struct Lexer {
    pub(crate) input: Vec<char>,
    pub(crate) position: usize,
    pub(crate) current_char: Option<char>,
}