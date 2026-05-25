use crate::ast::prelude::ErrorData;
use crate::interpreter::prelude::{Interpreter, RuntimeError, SharedInterner, Value};
use crate::traits::json::JsonParsable;
use crate::{define_builtin, expect_args, runtime_error};

pub fn setup_json_funcs(interpreter: &mut Interpreter, interner: &SharedInterner) {
    define_builtin!(interpreter, interner, crate::builtins::catalog::function::FROM_JSON.canonical => (_interpreter, arguments, span) {
        expect_args!(arguments, 1, span, "из_json");

        let json_text = arguments[0].value.as_str().ok_or_else(|| {
            runtime_error!(TypeError, span, "Функция 'из_json' ожидает строку")
        })?;

        let parsed = serde_json::from_str(json_text).map_err(|error| {
            runtime_error!(InvalidOperation, span, "Ошибка разбора JSON: {}", error)
        })?;

        Ok(Value::from_json(parsed))
    });

    define_builtin!(interpreter, interner, crate::builtins::catalog::function::TO_JSON.canonical => (_interpreter, arguments, span) {
            expect_args!(arguments, 1, span, "в_json");

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
