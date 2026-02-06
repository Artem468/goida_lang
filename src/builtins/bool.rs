use std::sync::Arc;
use crate::ast::prelude::ErrorData;
use crate::interpreter::prelude::{BuiltinFn, Interpreter, RuntimeError, SharedInterner, Value};

pub fn setup_bool_func(interpreter: &mut Interpreter, interner: &SharedInterner) {
    interpreter.builtins.insert(
        interner.write(|i| i.get_or_intern("логический")),
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
            let n: bool = arguments[0].clone().try_into()?;
            Ok(Value::Boolean(n))
        })),
    );
}