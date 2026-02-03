use crate::ast::prelude::ClassDefinition;
use crate::interpreter::prelude::{Module, RuntimeError, SharedInterner, Value};
use std::sync::{Arc, RwLock};
use string_interner::DefaultSymbol as Symbol;

pub trait CoreOperations {
    fn new(module: Module, interner: SharedInterner) -> Self;
    fn interpret(&mut self, module: Module) -> Result<(), RuntimeError>;
    fn resolve_symbol(&self, symbol: Symbol) -> Option<String>;
    fn intern_string(&self, s: &str) -> Symbol;
    fn load_imports(&mut self, module: &Module) -> Result<(), RuntimeError>;
    fn collect_imported_globals(&self, module: &Module) -> Result<Vec<(Symbol, Value)>, RuntimeError>;
    fn get_class_for_value(&self, value: &Value) -> Option<Arc<RwLock<ClassDefinition>>>;
}