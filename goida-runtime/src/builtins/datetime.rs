use crate::ast::prelude::{ErrorData, Span, Visibility};
use crate::builtins::registry::*;
use crate::interpreter::prelude::RuntimeClassDefinition;
use crate::interpreter::prelude::{
    BuiltinFn, CallArgListExt, CallArgValue, RuntimeError, SharedInterner, Value,
};
use crate::shared::SharedMut;
use crate::{bail_runtime, define_constructor, define_method, runtime_error};
use chrono::{DateTime, Datelike, Local, LocalResult, TimeZone, Timelike};
use std::sync::Arc;
use string_interner::DefaultSymbol as Symbol;

fn local_datetime(ms: i64, span: Span) -> Result<DateTime<Local>, RuntimeError> {
    match Local.timestamp_millis_opt(ms) {
        LocalResult::Single(datetime) => Ok(datetime),
        _ => bail_runtime!(InvalidOperation, span, "Date/time value is out of range"),
    }
}

fn shift_millis(
    current_ms: i64,
    amount: i64,
    unit_ms: i64,
    subtract: bool,
    span: Span,
) -> Result<i64, RuntimeError> {
    let delta = amount
        .checked_mul(unit_ms)
        .ok_or_else(|| runtime_error!(InvalidOperation, span, "Date/time arithmetic overflow"))?;
    let shifted = if subtract {
        current_ms.checked_sub(delta)
    } else {
        current_ms.checked_add(delta)
    };
    shifted.ok_or_else(|| runtime_error!(InvalidOperation, span, "Date/time arithmetic overflow"))
}

pub fn setup_datetime_class(
    interner_ref: &SharedInterner,
) -> (Symbol, SharedMut<RuntimeClassDefinition>) {
    let name_sym = interner_ref.write(|i| i.get_or_intern(class::DATETIME.names.canonical));
    let mut class_def = RuntimeClassDefinition::new(name_sym, Span::default());

    let ms_sym = interner_ref.write(|i| i.get_or_intern("_мс"));

    define_constructor!(class_def, (_, args, span) {
        let instance = match CallArgListExt::first_value(&args) {
            Some(Value::Object(inst)) => inst,
            _ => {
                return bail_runtime!(
                    TypeError,
                    span,
                    "Ошибка инициализации self"
                )
            }
        };

        let ms = if let Some(val) = CallArgListExt::get_value(&args, 1) {
            val.as_i64().ok_or_else(|| {
                runtime_error!(TypeError, span, "Аргумент должен быть числом")
            })?
        } else {
            Local::now().timestamp_millis()
        };

        local_datetime(ms, span)?;
        instance.write(|i| i.field_values.insert(ms_sym, Value::Number(ms)));

        Ok(Value::Empty)
    });

    // --- Вспомогательная функция: извлечь мс из self ---
    let get_ms = move |args: &Vec<CallArgValue>| -> Result<i64, RuntimeError> {
        if let Some(Value::Object(inst)) = CallArgListExt::first_value(args) {
            inst.read(|i| {
                i.field_values
                    .get(&ms_sym)
                    .and_then(|v| v.as_i64())
                    .ok_or_else(|| {
                        runtime_error!(
                            InvalidOperation,
                            Span::default(),
                            "Объект ДатаВремя поврежден"
                        )
                    })
            })
        } else {
            bail_runtime!(
                InvalidOperation,
                Span::default(),
                "Метод должен вызываться у объекта"
            )
        }
    };

    // --- Методы получения компонентов (год, месяц, день, час, минута, секунда) ---
    let components = [
        method::YEAR.canonical,
        method::MONTH.canonical,
        method::DAY.canonical,
        method::HOUR.canonical,
        method::MINUTE.canonical,
        method::SECOND.canonical,
    ];

    for name in components {
        let aliases = BUILTINS.method_names(name);
        let method_name = name.to_string();
        let method = BuiltinFn(Arc::new(move |_, args, span| {
            let ms = get_ms(&args)?;
            let dt = local_datetime(ms, span)?;
            let val = match method_name.as_str() {
                "год" => dt.year() as i64,
                "месяц" => dt.month() as i64,
                "день" => dt.day() as i64,
                "час" => dt.hour() as i64,
                "минута" => dt.minute() as i64,
                "секунда" => dt.second() as i64,
                _ => 0,
            };
            Ok(Value::Number(val))
        }));
        for alias in aliases {
            class_def.add_method(
                interner_ref.write(|i| i.get_or_intern(alias)),
                Visibility::Public,
                false,
                method.clone(),
            );
        }
    }

    for unit in DATETIME_UNITS {
        // --- Метод: ДОБАВИТЬ ---
        let add_aliases = unit.add.names;
        let ms_unit = unit.millis;
        let add_method = BuiltinFn(Arc::new(move |_, args, span| {
            let current_ms = get_ms(&args)?;
            let val = CallArgListExt::get_value(&args, 1)
                .and_then(|v| v.as_i64())
                .unwrap_or(0);

            let new_ms = shift_millis(current_ms, val, ms_unit, false, span)?;

            if let Some(Value::Object(inst)) = CallArgListExt::first_value(&args) {
                inst.write(|i| i.field_values.insert(ms_sym, Value::Number(new_ms)));
            }
            Ok(args[0].value.clone())
        }));
        for alias in add_aliases {
            class_def.add_method(
                interner_ref.write(|i| i.get_or_intern(alias)),
                Visibility::Public,
                false,
                add_method.clone(),
            );
        }

        // --- Метод: ВЫЧЕСТЬ ---
        let sub_aliases = unit.subtract.names;
        let ms_unit = unit.millis;
        let sub_method = BuiltinFn(Arc::new(move |_, args, span| {
            let current_ms = get_ms(&args)?;
            let val = CallArgListExt::get_value(&args, 1)
                .and_then(|v| v.as_i64())
                .unwrap_or(0);

            let new_ms = shift_millis(current_ms, val, ms_unit, true, span)?;

            if let Some(Value::Object(inst)) = CallArgListExt::first_value(&args) {
                inst.write(|i| i.field_values.insert(ms_sym, Value::Number(new_ms)));
            }
            Ok(args[0].value.clone())
        }));
        for alias in sub_aliases {
            class_def.add_method(
                interner_ref.write(|i| i.get_or_intern(alias)),
                Visibility::Public,
                false,
                sub_method.clone(),
            );
        }
    }

    // --- Метод: .сейчас() (стандартный вывод) ---
    define_method!(class_def, interner_ref, method::NOW.canonical => (_, args, _) {
        let now = Local::now();

        let pattern = match CallArgListExt::get_value(&args, 1) {
            Some(Value::Text(t)) => t.as_str(),
            _ => "%d.%m.%Y %H:%M:%S",
        };

        let formatted = now.format(pattern).to_string();
        Ok(Value::Text(formatted))
    });

    // --- Метод: .формат(шаблон) ---
    define_method!(class_def, interner_ref, method::FORMAT.canonical => (_, args, span) {
        let ms = get_ms(&args)?;
        let dt = local_datetime(ms, span)?;
        let pattern = CallArgListExt::get_value(&args, 1)
            .and_then(|v| v.as_str())
            .map(|s| s.as_str())
            .unwrap_or("%d.%m.%Y %H:%M:%S");

        Ok(Value::Text(dt.format(pattern).to_string()))
    });

    (name_sym, SharedMut::new(class_def))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_out_of_range_timestamp() {
        assert!(matches!(
            local_datetime(i64::MAX, Span::default()),
            Err(RuntimeError::InvalidOperation(_))
        ));
    }

    #[test]
    fn detects_datetime_arithmetic_overflow() {
        assert!(matches!(
            shift_millis(i64::MAX, 1, 1, false, Span::default()),
            Err(RuntimeError::InvalidOperation(_))
        ));
        assert!(matches!(
            shift_millis(0, i64::MAX, 2, true, Span::default()),
            Err(RuntimeError::InvalidOperation(_))
        ));
    }

    #[test]
    fn shifts_datetime_in_both_directions() {
        assert_eq!(
            shift_millis(10_000, 2, 1_000, false, Span::default()).unwrap(),
            12_000
        );
        assert_eq!(
            shift_millis(10_000, 2, 1_000, true, Span::default()).unwrap(),
            8_000
        );
    }
}
