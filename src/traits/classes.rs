use crate::ast::class::ClassDefinition;
use crate::ast::prelude::{FunctionDefinition, Program};
use crate::interpreter::prelude::{RuntimeError, Value};

pub trait InterpreterClasses {
    fn register_class(
        &mut self,
        class_def: &ClassDefinition,
        program: &Program,
    ) -> Result<(), RuntimeError>;
    fn call_method(
        &mut self,
        method: FunctionDefinition,
        arguments: Vec<Value>,
        this_obj: Value,
        program: &Program,
    ) -> Result<Value, RuntimeError>;
}