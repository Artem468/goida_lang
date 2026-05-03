use crate::ast::prelude::ErrorData;
use crate::interpreter::prelude::{Interpreter, RuntimeError, SharedInterner, Value};
use crate::{bail_runtime, define_builtin, runtime_error};

pub fn setup_bool_func(interpreter: &mut Interpreter, interner: &SharedInterner) {
    define_builtin!(interpreter, interner, "логический" => (_, arguments, span) {
        if arguments.len() != 1 {
            return bail_runtime!(
                InvalidOperation,
                span,
                "Функция 'логический' ожидает 1 аргумент, получено {}",
                arguments.len()
            )
        }

        let n: bool = arguments[0].value.clone().try_into()?;
        Ok(Value::Boolean(n))
    });
}
