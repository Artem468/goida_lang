use crate::ast::prelude::ErrorData;
use crate::interpreter::prelude::{BuiltinFn, Interpreter, RuntimeError, SharedInterner, Value};
use std::sync::Arc;

pub fn setup_number_func(interpreter: &mut Interpreter, interner: &SharedInterner) {
    interpreter.builtins.insert(
        interner.write(|i| i.get_or_intern("число")),
        BuiltinFn(Arc::new(move |_interpreter, arguments, span| {
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
        })),
    );
}
