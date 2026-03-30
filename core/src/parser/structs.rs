use crate::ast::prelude::ErrorData;
use crate::interpreter::prelude::{Module, SharedInterner};

#[derive(Debug)]
pub struct Parser {
    pub module: Module,
    pub(crate) interner: SharedInterner,
    pub(crate) nesting_level: usize,
}

#[derive(Debug)]
pub enum ParseError {
    TypeError(ErrorData),
    InvalidSyntax(ErrorData),
}
