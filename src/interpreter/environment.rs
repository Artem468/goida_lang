use std::collections::HashMap;
use crate::interpreter::prelude::{Environment, RuntimeError, Value};

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
    pub fn pop(self) -> Environment {
        match self.parent {
            Some(parent_box) => *parent_box,
            None => self
        }
    }

    pub(crate) fn define(&mut self, name: String, value: Value) {
        self.variables.insert(name, value);
    }

    pub(crate) fn get(&self, name: &str) -> Option<Value> {
        if let Some(value) = self.variables.get(name) {
            Some(value.clone())
        } else if let Some(parent) = &self.parent {
            parent.get(name)
        } else {
            None
        }
    }

    pub(crate) fn set(&mut self, name: &str, value: Value) -> Result<(), RuntimeError> {
        if self.variables.contains_key(name) {
            self.variables.insert(name.to_string(), value);
            Ok(())
        } else if let Some(parent) = &mut self.parent {
            parent.set(name, value)
        } else {
            self.variables.insert(name.to_string(), value);
            Ok(())
        }
    }
}