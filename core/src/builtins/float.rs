use crate::ast::prelude::ErrorData;
use crate::interpreter::prelude::{Interpreter, RuntimeError, SharedInterner, Value};
use crate::{bail_runtime, define_builtin, runtime_error};

pub fn setup_float_func(interpreter: &mut Interpreter, interner: &SharedInterner) {
    define_builtin!(interpreter, interner, "дробь" => (_, arguments, span) {
        if arguments.len() != 1 {
            return bail_runtime!(
                InvalidOperation,
                span,
                "Функция 'дробь' ожидает 1 аргумент, получено {}",
                arguments.len()
            );
        }
        let n: f64 = match arguments[0].value.clone().try_into() {
            Ok(i) => i,
            Err(err) => return bail_runtime!(InvalidOperation, span, "{}", err),
        };
        Ok(Value::Float(n))
    });
}
