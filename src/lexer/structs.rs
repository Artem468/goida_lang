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
    List,     // список
    Dict,     // словарь
    True,     // true
    False,    // false
    Print,    // print
    Input,    // input
    Import,   // import
    Push,     // добавить (в список)
    Pop,      // удалить последний (из списка)
    Remove,   // удалить по индексу/ключу
    Size,     // размер
    Contains, // содержит

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

#[derive(Debug, Clone, PartialEq)]
pub struct Span {
    pub line: usize,    // Номер строки (начиная с 1)
    pub column: usize,  // Позиция в строке (начиная с 1)
}

#[derive(Debug, Clone)]
pub struct TokenInfo {
    pub token: Token,
    pub span: Span,
}

pub struct Lexer {
    pub(crate) input: Vec<char>,
    pub(crate) position: usize,
    pub(crate) current_char: Option<char>,
    pub(crate) current_line: usize,
    pub(crate) current_column: usize,
}