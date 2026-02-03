use string_interner::DefaultSymbol as Symbol;
use crate::ast::prelude::{FunctionDefinition, Span};
use crate::ast::program::MethodType;
use crate::interpreter::prelude::{RuntimeError, Value};

pub trait InterpreterFunctions {
    fn call_function(
        &mut self,
        function: FunctionDefinition,
        arguments: Vec<Value>,
        current_module_id: Symbol,
        span: Span
    ) -> Result<Value, RuntimeError>;
    fn call_function_by_name(
        &mut self,
        name: Symbol,
        arguments: Vec<Value>,
        current_module_id: Symbol,
        span: Span
    ) -> Result<Value, RuntimeError>;
}

impl From<FunctionDefinition> for MethodType {
    fn from(func: FunctionDefinition) -> Self {
        MethodType::User(func)
    }
}