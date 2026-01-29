use crate::interpreter::prelude::{Environment, RuntimeError, Value};
use std::collections::HashMap;
use string_interner::{DefaultSymbol as Symbol};

impl Environment {
    pub(crate) fn new() -> Self {
        Environment {
            variables: HashMap::new(),
            parent: None,
        }
    }

    pub(crate) fn with_parent(parent: Environment) -> Self {
        Environment {
            variables: HashMap::new(),
            parent: Some(Box::new(parent)),
        }
    }

    pub(crate) fn define(&mut self, name: Symbol, value: Value) {
        self.variables.insert(name, value);
    }

    pub(crate) fn get(&self, name: &Symbol) -> Option<Value> {
        if let Some(value) = self.variables.get(name) {
            Some(value.clone())
        } else if let Some(parent) = &self.parent {
            parent.get(name)
        } else {
            None
        }
    }

    pub(crate) fn set(&mut self, name: Symbol, value: Value) -> Result<(), RuntimeError> {
        if self.variables.contains_key(&name) {
            self.variables.insert(name, value);
            Ok(())
        } else if let Some(parent) = &mut self.parent {
            parent.set(name, value)
        } else {
            Err(RuntimeError::UndefinedVariable("Переменная не найдена".into()))
        }
    }
}
