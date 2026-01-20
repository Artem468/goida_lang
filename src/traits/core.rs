use crate::ast::prelude::Program;
use crate::interpreter::prelude::{Module, RuntimeError};

pub trait CoreOperations {
    fn new(dir: std::path::PathBuf, program: Program) -> Self;
    fn into_module(self, program: Program) -> Module;
    fn interpret(&mut self, program: Program) -> Result<(), RuntimeError>;
}