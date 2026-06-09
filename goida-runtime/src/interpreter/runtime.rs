use crate::ast::prelude::{ErrorData, Span};
use crate::builtins::iterator::collect_iterator;
use crate::interpreter::prelude::{Interpreter, RuntimeError, Value};
use crate::{bail_runtime, runtime_error};
use string_interner::DefaultSymbol as Symbol;

impl Interpreter {
    pub(crate) fn assign_identifier(
        &mut self,
        name: Symbol,
        value: Value,
        module: Symbol,
        span: Span,
    ) -> Result<(), RuntimeError> {
        if self.try_assign_native_identifier(name, value.clone(), module, span)? {
            return Ok(());
        }

        let mut search = self.environment.clone();
        let mut target = None;
        let mut reached_function = false;
        loop {
            if search.read(|environment| environment.contains_assignment_target(name)) {
                target = Some(search.clone());
                break;
            }
            if search.read(|environment| environment.is_function) {
                reached_function = true;
                break;
            }
            let Some(parent) = search.read(|environment| environment.parent.clone()) else {
                break;
            };
            search = parent;
        }

        if let Some(target) = target {
            target.write(|environment| environment.set(name, value.clone(), span))?;
        } else if reached_function {
            self.environment
                .write(|environment| environment.define(name, value.clone()));
        } else if self
            .environment
            .write(|environment| environment.set(name, value.clone(), span))
            .is_err()
        {
            self.environment
                .write(|environment| environment.define(name, value.clone()));
        }

        if self
            .environment
            .read(|environment| environment.parent.is_none())
        {
            if let Some(module) = self.modules.get_mut(&module) {
                module.set_global(name, value);
            }
        }
        Ok(())
    }

    pub(crate) fn define_constant(
        &mut self,
        name: Symbol,
        value: Value,
        module: Symbol,
        span: Span,
    ) -> Result<(), RuntimeError> {
        if self
            .environment
            .read(|environment| environment.contains(name))
        {
            return bail_runtime!(InvalidOperation, span, "Cannot redefine a constant");
        }
        self.environment
            .write(|environment| environment.define_const(name, value.clone()));
        if self
            .environment
            .read(|environment| environment.parent.is_none())
        {
            if let Some(module) = self.modules.get_mut(&module) {
                module.set_global(name, value);
            }
        }
        Ok(())
    }

    pub(crate) fn iterable_values(
        &self,
        value: Value,
        span: Span,
    ) -> Result<Vec<Value>, RuntimeError> {
        match value {
            Value::List(values) => Ok(values.read(Clone::clone)),
            Value::Array(values) => Ok(values.as_ref().clone()),
            Value::Text(value) => Ok(value
                .chars()
                .map(|character| Value::Text(character.to_string()))
                .collect()),
            Value::Dict(values) => Ok(values.read(|values| {
                let mut keys = values.keys().cloned().collect::<Vec<_>>();
                keys.sort();
                keys.into_iter().map(Value::Text).collect()
            })),
            Value::Iterator(iterator) => collect_iterator(self, &iterator, span),
            _ => bail_runtime!(TypeError, span, "Value is not iterable"),
        }
    }

    pub(crate) fn join_thread_handle(
        &self,
        thread: &crate::interpreter::structs::RuntimeThread,
        span: Span,
    ) -> Result<Value, RuntimeError> {
        let handle = thread
            .handle
            .lock()
            .map_err(|_| runtime_error!(InvalidOperation, span, "Thread lock is poisoned"))?
            .take();
        match handle {
            Some(handle) => match handle.join() {
                Ok(Ok(())) => Ok(Value::Empty),
                Ok(Err(error)) => Err(error),
                Err(_) => bail_runtime!(Panic, span, "Thread panicked"),
            },
            None => Ok(Value::Empty),
        }
    }

    pub(crate) fn join_background_threads(
        &mut self,
        _module: Symbol,
        span: Span,
    ) -> Result<(), RuntimeError> {
        for thread in std::mem::take(&mut self.background_threads) {
            self.join_thread_handle(&thread, span)?;
        }
        Ok(())
    }
}
