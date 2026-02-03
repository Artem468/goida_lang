use crate::ast::prelude::{ClassDefinition, Span};
use crate::ast::program::MethodType;
use crate::interpreter::prelude::{RuntimeError, Value};
use std::rc::Rc;
use string_interner::DefaultSymbol as Symbol;

pub trait InterpreterClasses {
    fn call_method(
        &mut self,
        method: MethodType,
        arguments: Vec<Value>,
        this_obj: Value,
        current_module_id: Symbol,
        span: Span
    ) -> Result<Value, RuntimeError>;

    fn set_class_module(
        &self,
        class_def: Rc<ClassDefinition>,
        module: Symbol,
    ) -> Rc<ClassDefinition>;
}

impl MethodType {
    pub fn get_module(&self) -> Option<Symbol> {
        match self {
            MethodType::User(func) => func.module,
            MethodType::Native(_) => None, // У нативных методов нет модуля в AST
        }
    }
}