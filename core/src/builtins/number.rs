use crate::ast::prelude::ErrorData;
use crate::define_builtin;
use crate::interpreter::prelude::{Interpreter, RuntimeError, SharedInterner, Value};

pub fn setup_number_func(interpreter: &mut Interpreter, interner: &SharedInterner) {
    define_builtin!(interpreter, interner, "число" => (_interpreter, arguments, span) {
        if arguments.len() != 1 {
            return Err(RuntimeError::InvalidOperation(ErrorData::new(
                span,
                format!(
                    "Функция 'число' ожидает 1 аргумент, получено {}",
                    arguments.len()
                ),
            )));
        }

        let n: i64 = match arguments[0].value.clone().try_into() {
            Ok(i) => i,
            Err(err) => return Err(RuntimeError::InvalidOperation(ErrorData::new(span, err))),
        };

        Ok(Value::Number(n))
    });
}
