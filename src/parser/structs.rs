use crate::lexer::structs::TokenInfo;
use crate::ast::*;

pub struct Parser {
    pub(crate) program: Program,
    pub(crate) tokens: Vec<TokenInfo>,
    pub(crate) current: usize,
}

#[derive(Debug)]
pub enum ParseError {
    UnexpectedToken(String),
    InternalError(String),
    TypeError(String),
}
