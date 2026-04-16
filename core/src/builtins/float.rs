use crate::ast::prelude::ErrorData;
use crate::define_builtin;
use crate::interpreter::prelude::{Interpreter, RuntimeError, SharedInterner, Value};

pub fn setup_float_func(interpreter: &mut Interpreter, interner: &SharedInterner) {
    define_builtin!(interpreter, interner, "дробь" => (_, arguments, span) {
        if arguments.len() != 1 {
            return Err(RuntimeError::InvalidOperation(ErrorData::new(
                span,
                format!(
                    "Функция 'дробь' ожидает 1 аргумент, получено {}",
                    arguments.len()
                ),
            )));
        }
        let n: f64 = match arguments[0].value.clone().try_into() {
            Ok(i) => i,
            Err(err) => return Err(RuntimeError::InvalidOperation(ErrorData::new(span, err))),
        };
        Ok(Value::Float(n))
    });
}
