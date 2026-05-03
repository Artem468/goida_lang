use crate::ast::prelude::ErrorData;
use crate::interpreter::prelude::{Interpreter, RuntimeError, SharedInterner, Value};
use crate::traits::json::JsonParsable;
use crate::{bail_runtime, define_builtin, runtime_error};

pub fn setup_json_funcs(interpreter: &mut Interpreter, interner: &SharedInterner) {
    define_builtin!(interpreter, interner, "из_json" => (_interpreter, arguments, span) {
        if arguments.len() != 1 {
            return bail_runtime!(InvalidOperation, span, "Функция 'из_json' ожидает 1 аргумент, получено {}", arguments.len())
        }

        let json_text = arguments[0].value.as_str().ok_or_else(|| {
            runtime_error!(TypeError, span, "Функция 'из_json' ожидает строку")
        })?;

        let parsed = serde_json::from_str(json_text).map_err(|error| {
            runtime_error!(InvalidOperation, span, "Ошибка разбора JSON: {}", error)
        })?;

        Ok(Value::from_json(parsed))
    });

    define_builtin!(interpreter, interner, "в_json" => (_interpreter, arguments, span) {
            if arguments.len() != 1 {
                return bail_runtime!(InvalidOperation, span, "Функция 'в_json' ожидает 1 аргумент, получено {}", arguments.len())
            }

            let json_value = arguments[0].value.to_json().map_err(|error| {
                runtime_error!(InvalidOperation, span, "Ошибка сериализации JSON: {}", error)
            })?;

            serde_json::to_string(&json_value)
                .map(Value::Text)
                .map_err(|error| {
                    runtime_error!(InvalidOperation, span, "Ошибка сериализации JSON: {}", error)
                })
    });
}
