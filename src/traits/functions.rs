use crate::ast::prelude::{FunctionDefinition, Program};
use crate::interpreter::prelude::{RuntimeError, Value};

pub trait InterpreterFunctions {
    fn call_function(
        &mut self,
        function: FunctionDefinition,
        arguments: Vec<Value>,
        program: &Program,
    ) -> Result<Value, RuntimeError>;
    fn call_function_by_name(
        &mut self,
        name: &str,
        arguments: Vec<Value>,
        program: &Program,
    ) -> Result<Value, RuntimeError>;
}