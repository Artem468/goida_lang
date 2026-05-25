use crate::ast::prelude::{ClassDefinition, ErrorData, Span, Visibility};
use crate::interpreter::prelude::{
    BuiltinFn, CallArgListExt, CallArgValue, RuntimeError, SharedInterner, Value,
};
use crate::shared::SharedMut;
use crate::{bail_runtime, define_constructor, define_method, runtime_error};
use chrono::{Datelike, Local, TimeZone, Timelike};
use std::sync::Arc;
use string_interner::DefaultSymbol as Symbol;

pub fn setup_datetime_class(interner_ref: &SharedInterner) -> (Symbol, SharedMut<ClassDefinition>) {
    let name_sym = interner_ref
        .write(|i| i.get_or_intern(crate::builtins::catalog::class::DATETIME.names.canonical));
    let mut class_def = ClassDefinition::new(name_sym, Span::default());

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
        crate::builtins::catalog::method::YEAR.canonical,
        crate::builtins::catalog::method::MONTH.canonical,
        crate::builtins::catalog::method::DAY.canonical,
        crate::builtins::catalog::method::HOUR.canonical,
        crate::builtins::catalog::method::MINUTE.canonical,
        crate::builtins::catalog::method::SECOND.canonical,
    ];

    for name in components {
        let aliases = crate::builtins::catalog::method_names(name);
        let method_name = name.to_string();
        let method = BuiltinFn(Arc::new(move |_, args, _| {
            let ms = get_ms(&args)?;
            let dt = Local.timestamp_millis_opt(ms).unwrap();
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

    for unit in crate::builtins::catalog::DATETIME_UNITS {
        // --- Метод: ДОБАВИТЬ ---
        let add_aliases = unit.add.names;
        let ms_unit = unit.millis;
        let add_method = BuiltinFn(Arc::new(move |_, args, _span| {
            let current_ms = get_ms(&args)?;
            let val = CallArgListExt::get_value(&args, 1)
                .and_then(|v| v.as_i64())
                .unwrap_or(0);

            let new_ms = current_ms + (val * ms_unit);

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
        let sub_method = BuiltinFn(Arc::new(move |_, args, _span| {
            let current_ms = get_ms(&args)?;
            let val = CallArgListExt::get_value(&args, 1)
                .and_then(|v| v.as_i64())
                .unwrap_or(0);

            let new_ms = current_ms - (val * ms_unit);

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
    define_method!(class_def, interner_ref, crate::builtins::catalog::method::NOW.canonical => (_, args, _) {
        let now = Local::now();

        let pattern = match CallArgListExt::get_value(&args, 1) {
            Some(Value::Text(t)) => t.as_str(),
            _ => "%d.%m.%Y %H:%M:%S",
        };

        let formatted = now.format(pattern).to_string();
        Ok(Value::Text(formatted))
    });

    // --- Метод: .формат(шаблон) ---
    define_method!(class_def, interner_ref, crate::builtins::catalog::method::FORMAT.canonical => (_, args, _) {
        let ms = get_ms(&args)?;
        let dt = Local.timestamp_millis_opt(ms).unwrap();
        let pattern = CallArgListExt::get_value(&args, 1)
            .and_then(|v| v.as_str())
            .map(|s| s.as_str())
            .unwrap_or("%d.%m.%Y %H:%M:%S");

        Ok(Value::Text(dt.format(pattern).to_string()))
    });

    (name_sym, SharedMut::new(class_def))
}
