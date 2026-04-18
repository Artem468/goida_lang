use crate::ast::prelude::ClassDefinition;
use crate::interpreter::prelude::{Module, RuntimeError, SharedInterner, Value};
use crate::shared::SharedMut;
use string_interner::DefaultSymbol as Symbol;

pub trait CoreOperations {
    fn new(interner: SharedInterner) -> Self;
    fn load_start_module(&mut self, main_module: Module) -> &mut Self;
    fn interpret(&mut self, module: Symbol) -> Result<(), RuntimeError>;
    fn resolve_import_alias_symbol(
        &self,
        current_module: &Module,
        alias: Symbol,
    ) -> Option<Symbol>;
    fn resolve_symbol(&self, symbol: Symbol) -> Option<String>;
    fn intern_string(&self, s: &str) -> Symbol;
    fn get_class_for_value(&self, value: &Value) -> Option<SharedMut<ClassDefinition>>;
    fn get_file_path(&self, module_id: &Symbol) -> String;
}
