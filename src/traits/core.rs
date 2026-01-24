use string_interner::DefaultSymbol as Symbol;
use crate::interpreter::prelude::{Module, RuntimeError, SharedInterner};

pub trait CoreOperations {
    fn new(module: Module, interner: SharedInterner) -> Self;
    fn interpret(&mut self, module: Module) -> Result<(), RuntimeError>;
    fn resolve_symbol(&self, symbol: Symbol) -> Option<String>;
    fn intern_string(&self, s: &str) -> Symbol;
}