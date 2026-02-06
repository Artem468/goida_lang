use std::sync::Arc;
use crate::ast::prelude::ErrorData;
use crate::interpreter::prelude::{BuiltinFn, Interpreter, RuntimeError, SharedInterner, Value};

pub fn setup_float_func(interpreter: &mut Interpreter, interner: &SharedInterner) {
    interpreter.builtins.insert(
        interner.write(|i| i.get_or_intern("дробь")),
        BuiltinFn(Arc::new(move |_interpreter, arguments, span| {
            if arguments.len() != 1 {
                return Err(RuntimeError::InvalidOperation(ErrorData::new(
                    span,
                    format!(
                        "Функция 'дробь' ожидает 1 аргумент, получено {}",
                        arguments.len()
                    ),
                )));
            }
            let n: f64 = match arguments[0].clone().try_into() {
                Ok(i) => i,
                Err(err) => {
                    return Err(RuntimeError::InvalidOperation(ErrorData::new(span, err)))
                }
            };
            Ok(Value::Float(n))
        })),
    );
}