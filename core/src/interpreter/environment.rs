use crate::ast::prelude::{ErrorData, Span};
use crate::interpreter::prelude::{Environment, Interpreter, RuntimeError, Value};
use crate::shared::SharedMut;
use std::collections::HashMap;
use string_interner::DefaultSymbol as Symbol;
use crate::{bail_runtime, runtime_error};

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
        bail_runtime!(
            UndefinedVariable,
            span,
            "Переменная не найдена"
        )
    }
}

impl Interpreter {
    pub(crate) fn scoped_environment<R>(
        &mut self,
        environment: Environment,
        execute: impl FnOnce(&mut Self) -> Result<R, RuntimeError>,
    ) -> Result<R, RuntimeError> {
        let previous_env = self.environment.clone();
        self.environment = SharedMut::new(environment);
        let result = execute(self);
        self.environment = previous_env;
        result
    }

    pub(crate) fn scoped_child_environment<R>(
        &mut self,
        configure: impl FnOnce(&mut Environment),
        execute: impl FnOnce(&mut Self) -> Result<R, RuntimeError>,
    ) -> Result<R, RuntimeError> {
        let mut environment = Environment::with_parent(self.environment.clone());
        configure(&mut environment);
        self.scoped_environment(environment, execute)
    }
}
