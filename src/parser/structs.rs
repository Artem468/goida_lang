use crate::ast::prelude::ErrorData;
use crate::interpreter::prelude::{Module, SharedInterner};

#[derive(Debug)]
pub struct Parser {
    pub(crate) module: Module,
    pub(crate) interner: SharedInterner,
}

#[derive(Debug)]
pub enum ParseError {
    UnexpectedToken(ErrorData),
    TypeError(ErrorData),
}