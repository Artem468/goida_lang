use crate::ast::prelude::{ErrorData, Span};
use crate::interpreter::prelude::{Environment, Interpreter, RuntimeError, Value};
use crate::shared::SharedMut;
use crate::{bail_runtime, runtime_error};
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
        bail_runtime!(UndefinedVariable, span, "Переменная не найдена")
    }
}

impl Interpreter {
    fn enter_environment(&mut self, environment: SharedMut<Environment>) -> EnvironmentGuard {
        let previous = std::mem::replace(&mut self.environment, environment);
        EnvironmentGuard {
            environment: &mut self.environment,
            previous: Some(previous),
        }
    }

    pub(crate) fn scoped_environment<R>(
        &mut self,
        environment: Environment,
        execute: impl FnOnce(&mut Self) -> Result<R, RuntimeError>,
    ) -> Result<R, RuntimeError> {
        let _guard = self.enter_environment(SharedMut::new(environment));
        execute(self)
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

    pub(crate) fn preserving_environment<R>(
        &mut self,
        execute: impl FnOnce(&mut Self) -> Result<R, RuntimeError>,
    ) -> Result<R, RuntimeError> {
        let _guard = self.enter_environment(self.environment.clone());
        execute(self)
    }
}

struct EnvironmentGuard {
    environment: *mut SharedMut<Environment>,
    previous: Option<SharedMut<Environment>>,
}

impl Drop for EnvironmentGuard {
    fn drop(&mut self) {
        if let Some(previous) = self.previous.take() {
            // SAFETY: EnvironmentGuard is created only from Interpreter methods and is not exposed.
            // The pointed field belongs to that interpreter and remains valid for the guarded scope.
            unsafe {
                *self.environment = previous;
            }
        }
    }
}
