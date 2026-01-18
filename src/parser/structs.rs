use crate::ast::prelude::Program;

#[derive(Debug)]
pub struct Parser {
    pub(crate) program: Program
}

#[derive(Debug)]
pub enum ParseError {
    UnexpectedToken(String),
    InternalError(String),
    TypeError(String),
}
