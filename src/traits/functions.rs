use string_interner::DefaultSymbol as Symbol;
use crate::ast::prelude::{FunctionDefinition};
use crate::interpreter::prelude::{RuntimeError, Value};

pub trait InterpreterFunctions {
    fn call_function(
        &mut self,
        function: FunctionDefinition,
        arguments: Vec<Value>,
        current_module_id: Symbol,
    ) -> Result<Value, RuntimeError>;
    fn call_function_by_name(
        &mut self,
        name: Symbol,
        arguments: Vec<Value>,
        current_module_id: Symbol,
    ) -> Result<Value, RuntimeError>;
}