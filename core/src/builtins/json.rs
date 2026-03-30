use crate::ast::prelude::ErrorData;
use crate::interpreter::prelude::{BuiltinFn, Interpreter, RuntimeError, SharedInterner, Value};
use crate::traits::json::JsonParsable;
use std::sync::Arc;

pub fn setup_json_funcs(interpreter: &mut Interpreter, interner: &SharedInterner) {
    interpreter.builtins.insert(
        interner.write(|i| i.get_or_intern("из_json")),
        BuiltinFn(Arc::new(move |_interpreter, arguments, span| {
            if arguments.len() != 1 {
                return Err(RuntimeError::InvalidOperation(ErrorData::new(
                    span,
                    format!(
                        "Функция 'из_json' ожидает 1 аргумент, получено {}",
                        arguments.len()
                    ),
                )));
            }

            let json_text = arguments[0].value.as_str().ok_or_else(|| {
                RuntimeError::TypeError(ErrorData::new(
                    span,
                    "Функция 'из_json' ожидает строку".into(),
                ))
            })?;

            let parsed = serde_json::from_str(json_text).map_err(|error| {
                RuntimeError::InvalidOperation(ErrorData::new(
                    span,
                    format!("Ошибка разбора JSON: {}", error),
                ))
            })?;

            Ok(Value::from_json(parsed))
        })),
    );

    interpreter.builtins.insert(
        interner.write(|i| i.get_or_intern("в_json")),
        BuiltinFn(Arc::new(move |_interpreter, arguments, span| {
            if arguments.len() != 1 {
                return Err(RuntimeError::InvalidOperation(ErrorData::new(
                    span,
                    format!(
                        "Функция 'в_json' ожидает 1 аргумент, получено {}",
                        arguments.len()
                    ),
                )));
            }

            let json_value = arguments[0].value.to_json().map_err(|error| {
                RuntimeError::InvalidOperation(ErrorData::new(
                    span,
                    format!("Ошибка сериализации JSON: {}", error),
                ))
            })?;

            serde_json::to_string(&json_value)
                .map(Value::Text)
                .map_err(|error| {
                    RuntimeError::InvalidOperation(ErrorData::new(
                        span,
                        format!("Ошибка сериализации JSON: {}", error),
                    ))
                })
        })),
    );
}
