use crate::ast::prelude::{ClassDefinition, ExprId, FunctionDefinition, Program, StmtId};
use crate::interpreter::structs::{Module, RuntimeError, Value};

pub trait StatementExecutor {
    fn execute_statement(&mut self, stmt_id: StmtId, program: &Program)
                         -> Result<(), RuntimeError>;
}

pub trait ExpressionEvaluator {
    fn evaluate_expression(
        &mut self,
        expr_id: ExprId,
        program: &Program,
    ) -> Result<Value, RuntimeError>;
}

pub trait CoreOperations {
    fn new(dir: std::path::PathBuf, program: Program) -> Self;
    fn into_module(self, program: Program) -> Module;
    fn interpret(&mut self, program: Program) -> Result<(), RuntimeError>;
}

pub trait ValueOperations {
    fn add_values(&self, left: Value, right: Value) -> Result<Value, RuntimeError>;
    fn subtract_values(&self, left: Value, right: Value) -> Result<Value, RuntimeError>;
    fn multiply_values(&self, left: Value, right: Value) -> Result<Value, RuntimeError>;
    fn divide_values(&self, left: Value, right: Value) -> Result<Value, RuntimeError>;
    fn modulo_values(&self, left: Value, right: Value) -> Result<Value, RuntimeError>;
    fn compare_greater(&self, left: Value, right: Value) -> Result<Value, RuntimeError>;
    fn compare_less(&self, left: Value, right: Value) -> Result<Value, RuntimeError>;
    fn compare_greater_equal(&self, left: Value, right: Value) -> Result<Value, RuntimeError>;
    fn compare_less_equal(&self, left: Value, right: Value) -> Result<Value, RuntimeError>;
}

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