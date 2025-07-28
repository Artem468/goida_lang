use crate::lexer::structs::{TokenInfo};

pub struct Parser {
    pub(crate) name: String,
    pub(crate) tokens: Vec<TokenInfo>,
    pub(crate) current: usize,
}

#[derive(Debug)]
pub enum ParseError {
    UnexpectedToken(String),
}