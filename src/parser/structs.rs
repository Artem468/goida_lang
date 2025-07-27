use crate::lexer::structs::Token;

pub struct Parser {
    pub(crate) tokens: Vec<Token>,
    pub(crate) current: usize,
}

#[derive(Debug)]
pub enum ParseError {
    UnexpectedToken(String),
}