use crate::ast::prelude::{ErrorData, Span};
use crate::interpreter::prelude::{Environment, Interpreter, RuntimeError, Value, VariableSlot};
use crate::shared::SharedMut;
use crate::{bail_runtime, runtime_error};
use std::collections::{HashMap, HashSet};
use string_interner::DefaultSymbol as Symbol;

use VariableSlot::{GlobalSlot, LocalSlot, UpvalueSlot};

impl Environment {
    pub(crate) fn new() -> Self {
        Environment {
            slots: Vec::new(),
            upvalues: Vec::new(),
            bindings: HashMap::new(),
            constants: HashSet::new(),
            parent: None,
            is_function: false,
        }
    }

    pub(crate) fn with_parent(parent: SharedMut<Environment>) -> Self {
        Self::child(parent, false)
    }

    pub(crate) fn with_parent_function(parent: SharedMut<Environment>) -> Self {
        Self::child(parent, true)
    }

    fn child(parent: SharedMut<Environment>, is_function: bool) -> Self {
        Environment {
            slots: Vec::new(),
            upvalues: Vec::new(),
            bindings: HashMap::new(),
            constants: HashSet::new(),
            parent: Some(parent),
            is_function,
        }
    }

    pub(crate) fn define(&mut self, name: Symbol, value: Value) {
        if let Some(LocalSlot(slot)) = self.bindings.get(&name).copied() {
            self.slots[slot as usize].write(|target| *target = value);
            return;
        }

        let slot = LocalSlot(self.slots.len() as u32);
        self.slots.push(SharedMut::new(value));
        self.bindings.insert(name, slot);
    }

    pub(crate) fn define_const(&mut self, name: Symbol, value: Value) {
        self.define(name, value);
        if let Some(slot) = self.bindings.get(&name).copied() {
            self.constants.insert(slot);
        }
    }

    pub(crate) fn contains(&self, name: Symbol) -> bool {
        self.bindings.contains_key(&name)
            || self
                .parent
                .as_ref()
                .is_some_and(|parent| parent.read(|parent| parent.contains(name)))
    }

    pub(crate) fn contains_assignment_target(&self, name: Symbol) -> bool {
        matches!(self.bindings.get(&name), Some(LocalSlot(_)))
            || !self.is_function && self.contains(name)
    }

    pub(crate) fn get(&self, name: &Symbol) -> Option<Value> {
        if let Some(slot) = self.bindings.get(name).copied() {
            return self.get_slot(slot);
        }
        self.parent
            .as_ref()
            .and_then(|parent| parent.read(|parent| parent.get(name)))
    }

    fn get_slot(&self, slot: VariableSlot) -> Option<Value> {
        self.slot_value(slot).map(|value| value.read(Clone::clone))
    }

    fn slot_value(&self, slot: VariableSlot) -> Option<&SharedMut<Value>> {
        match slot {
            LocalSlot(slot) => self.slots.get(slot as usize),
            UpvalueSlot(slot) => self.upvalues.get(slot as usize),
            GlobalSlot(_) => None,
        }
    }

    pub(crate) fn set(
        &mut self,
        name: Symbol,
        value: Value,
        span: Span,
    ) -> Result<(), RuntimeError> {
        if let Some(slot) = self.bindings.get(&name).copied() {
            if self.constants.contains(&slot) {
                return bail_runtime!(InvalidOperation, span, "Нельзя изменить константу");
            }
            let Some(target) = self.slot_value(slot).cloned() else {
                return bail_runtime!(UndefinedVariable, span, "Переменная не найдена");
            };
            target.write(|current| *current = value);
            return Ok(());
        }
        if let Some(parent) = &self.parent {
            return parent.write(|parent| parent.set(name, value, span));
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

    pub(crate) fn scoped_child_function_environment<R>(
        &mut self,
        configure: impl FnOnce(&mut Environment),
        execute: impl FnOnce(&mut Self) -> Result<R, RuntimeError>,
    ) -> Result<R, RuntimeError> {
        let mut environment = Environment::with_parent_function(self.environment.clone());
        configure(&mut environment);
        self.scoped_environment(environment, execute)
    }

    pub(crate) fn scoped_method_context<R>(
        &mut self,
        execute: impl FnOnce(&mut Self) -> Result<R, RuntimeError>,
    ) -> Result<R, RuntimeError> {
        self.method_depth += 1;
        let _guard = MethodContextGuard {
            method_depth: &mut self.method_depth,
        };
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
            // SAFETY: the guard cannot outlive the interpreter field it points to.
            unsafe {
                *self.environment = previous;
            }
        }
    }
}

struct MethodContextGuard {
    method_depth: *mut usize,
}

impl Drop for MethodContextGuard {
    fn drop(&mut self) {
        // SAFETY: the guard cannot outlive the interpreter field it points to.
        unsafe {
            *self.method_depth -= 1;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use string_interner::{DefaultSymbol, Symbol as _};

    fn symbol(index: usize) -> DefaultSymbol {
        DefaultSymbol::try_from_usize(index).unwrap()
    }

    #[test]
    fn child_lazily_reads_and_updates_parent() {
        let name = symbol(0);
        let parent = SharedMut::new(Environment::new());
        parent.write(|environment| environment.define(name, Value::Number(1)));

        let mut child = Environment::with_parent(parent.clone());
        assert_eq!(child.bindings.get(&name), None);
        assert_eq!(child.get(&name), Some(Value::Number(1)));

        child.set(name, Value::Number(2), Span::default()).unwrap();
        assert_eq!(
            parent.read(|environment| environment.get(&name)),
            Some(Value::Number(2))
        );
    }

    #[test]
    fn local_definition_shadows_inherited_slot() {
        let name = symbol(0);
        let parent = SharedMut::new(Environment::new());
        parent.write(|environment| environment.define(name, Value::Number(1)));

        let mut child = Environment::with_parent(parent.clone());
        child.define(name, Value::Number(2));

        assert_eq!(child.bindings.get(&name), Some(&LocalSlot(0)));
        assert_eq!(child.get(&name), Some(Value::Number(2)));
        assert_eq!(
            parent.read(|environment| environment.get(&name)),
            Some(Value::Number(1))
        );
    }

    #[test]
    fn inherited_constant_cannot_be_updated() {
        let name = symbol(0);
        let parent = SharedMut::new(Environment::new());
        parent.write(|environment| environment.define_const(name, Value::Number(1)));

        let mut child = Environment::with_parent(parent);
        assert!(child.set(name, Value::Number(2), Span::default()).is_err());
    }
}
