use crate::ast::prelude::{ErrorData, Span};
use crate::interpreter::prelude::{Environment, RuntimeError, Value};
use crate::shared::SharedMut;
use std::collections::HashMap;
use string_interner::DefaultSymbol as Symbol;

impl Environment {
    pub(crate) fn new() -> Self {
        Environment {
            variables: HashMap::new(),
            parent: None,
        }
    }

    pub(crate) fn with_parent(parent: SharedMut<Environment>) -> Self {
        Environment {
            variables: HashMap::new(),
            parent: Some(parent),
        }
    }

    pub(crate) fn define(&mut self, name: Symbol, value: Value) {
        self.variables.insert(name, value);
    }

    pub(crate) fn get(&self, name: &Symbol) -> Option<Value> {
        if let Some(value) = self.variables.get(name) {
            return Some(value.clone());
        }

        if let Some(parent_shared) = &self.parent {
            return parent_shared.read(|parent| parent.get(name));
        }

        None
    }

    pub(crate) fn set(
        &mut self,
        name: Symbol,
        value: Value,
        span: Span,
    ) -> Result<(), RuntimeError> {
        if self.variables.contains_key(&name) {
            self.variables.insert(name, value);
            return Ok(());
        }

        if let Some(parent_shared) = &self.parent {
            return parent_shared.write(|parent| parent.set(name, value, span));
        }

        Err(RuntimeError::UndefinedVariable(ErrorData::new(
            span,
            "Переменная не найдена".into(),
        )))
    }
}
