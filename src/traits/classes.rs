use crate::ast::prelude::{ClassDefinition, FunctionDefinition};
use crate::interpreter::prelude::{RuntimeError, Value};
use std::rc::Rc;
use string_interner::DefaultSymbol as Symbol;

pub trait InterpreterClasses {
    fn register_class(
        &mut self,
        class_def: Rc<ClassDefinition>,
        current_module_id: Symbol,
    ) -> Result<(), RuntimeError>;
    fn call_method(
        &mut self,
        method: FunctionDefinition,
        arguments: Vec<Value>,
        this_obj: Value,
        current_module_id: Symbol,
    ) -> Result<Value, RuntimeError>;
}