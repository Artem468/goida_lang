use crate::ast::prelude::Span;
use crate::interpreter::prelude::{
    CallArgValue, ClassInstance, RuntimeClassDefinition, RuntimeError, RuntimeMethodType, Value,
};
use crate::shared::SharedMut;
use string_interner::DefaultSymbol as Symbol;

pub trait InterpreterClasses {
    fn call_method(
        &mut self,
        method: RuntimeMethodType,
        arguments: Vec<CallArgValue>,
        this_obj: Value,
        current_module_id: Symbol,
        span: Span,
    ) -> Result<Value, RuntimeError>;

    fn set_class_module(
        &self,
        class_def: SharedMut<RuntimeClassDefinition>,
        module: Symbol,
    ) -> SharedMut<RuntimeClassDefinition>;
}

impl RuntimeMethodType {
    pub fn get_module(&self) -> Option<Symbol> {
        match self {
            RuntimeMethodType::User(func) => func.module,
            RuntimeMethodType::Native(_) => None, // У нативных методов нет модуля в AST
        }
    }
}

impl PartialEq for RuntimeClassDefinition {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name && self.span == other.span
    }
}

impl PartialEq for ClassInstance {
    fn eq(&self, other: &Self) -> bool {
        self.class_name == other.class_name
    }
}
