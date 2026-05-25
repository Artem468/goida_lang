use crate::ast::prelude::ErrorData;
use crate::interpreter::prelude::{Module, SharedInterner};

#[derive(Debug)]
/// Stateful parser for a single module.
pub struct Parser {
    /// Module under construction.
    pub module: Module,
    pub(crate) interner: SharedInterner,
}

#[derive(Debug)]
/// Errors produced while parsing or validating source.
pub enum ParseError {
    TypeError(ErrorData),
    InvalidSyntax(ErrorData),
    ImportError(ErrorData),
}
