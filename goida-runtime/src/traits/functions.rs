use crate::ast::prelude::{FunctionDefinition, Span};
use crate::interpreter::prelude::{CallArgValue, RuntimeError, RuntimeMethodType, Value};
use std::sync::Arc;
use string_interner::DefaultSymbol as Symbol;

pub trait InterpreterFunctions {
    fn call_function(
        &mut self,
        function: Arc<FunctionDefinition>,
        arguments: Vec<CallArgValue>,
        current_module_id: Symbol,
        span: Span,
    ) -> Result<Value, RuntimeError>;
    fn call_function_by_name(
        &mut self,
        name: Symbol,
        arguments: Vec<CallArgValue>,
        current_module_id: Symbol,
        span: Span,
    ) -> Result<Value, RuntimeError>;
}

impl From<FunctionDefinition> for RuntimeMethodType {
    fn from(func: FunctionDefinition) -> Self {
        RuntimeMethodType::User(Arc::new(func))
    }
}
