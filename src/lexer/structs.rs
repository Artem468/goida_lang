use crate::ast::Span;

#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    // Ключевые слова
    Let,      // пусть
    If,       // если
    Else,     // иначе
    While,    // пока
    For,      // для
    Function, // функция
    Return,   // вернуть
    Number,   // число
    Float,    // дробь
    Text,     // текст
    Boolean,  // логический
    List,     // список
    Dict,     // словарь
    True,     // истина
    False,    // ложь
    Print,    // печать
    Input,    // ввод
    Import,   // подключить
    Push,     // добавить
    Pop,      // извлечь
    Remove,   // удалить
    Size,     // длина
    Contains, // содержит

    // Литералы
    NumericalLiteral(i64),
    FloatLiteral(f64),
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

#[derive(Debug, Clone)]
pub struct TokenInfo {
    pub token: Token,
    pub span: Span,
}

pub struct Lexer {
    pub(crate) input: Vec<char>,
    pub(crate) position: usize,
    pub(crate) current_char: Option<char>,
    pub(crate) current_line: u32,
    pub(crate) current_column: u32,
}
