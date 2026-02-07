use crate::ast::prelude::ClassDefinition;
use crate::interpreter::prelude::{Module, RuntimeError, Value};
use crate::shared::SharedMut;
use string_interner::DefaultSymbol as Symbol;

pub trait CoreOperations {
    fn new() -> Self;
    fn load_start_module(&mut self, main_module: Module) -> &mut Self;
    fn interpret(&mut self, module: Symbol) -> Result<(), RuntimeError>;
    fn resolve_symbol(&self, symbol: Symbol) -> Option<String>;
    fn intern_string(&self, s: &str) -> Symbol;
    fn load_imports(&mut self, module: &Module) -> Result<(), RuntimeError>;
    fn collect_imported_globals(&self, module: &Module) -> Result<Vec<(Symbol, Value)>, RuntimeError>;
    fn get_class_for_value(&self, value: &Value) -> Option<SharedMut<ClassDefinition>>;
    fn get_file_path(&self, module_id: &Symbol) -> String;
}