use crate::ast::prelude::{ClassDefinition, Span};
use crate::ast::program::MethodType;
use crate::interpreter::prelude::{ClassInstance, RuntimeError, Value};
use crate::shared::SharedMut;
use string_interner::DefaultSymbol as Symbol;

pub trait InterpreterClasses {
    fn call_method(
        &mut self,
        method: MethodType,
        arguments: Vec<Value>,
        this_obj: Value,
        current_module_id: Symbol,
        span: Span,
    ) -> Result<Value, RuntimeError>;

    fn set_class_module(
        &self,
        class_def: SharedMut<ClassDefinition>,
        module: Symbol,
    ) -> SharedMut<ClassDefinition>;
}

impl MethodType {
    pub fn get_module(&self) -> Option<Symbol> {
        match self {
            MethodType::User(func) => func.module,
            MethodType::Native(_) => None, // У нативных методов нет модуля в AST
        }
    }
}

impl PartialEq for ClassDefinition {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name && self.span == other.span
    }
}

impl PartialEq for ClassInstance {
    fn eq(&self, other: &Self) -> bool {
        self.class_name == other.class_name
    }
}
