use crate::ast::prelude::{ClassDefinition, ErrorData, Span, Visibility};
use crate::interpreter::prelude::{BuiltinFn, RuntimeError, SharedInterner, Value};
use crate::shared::SharedMut;
use chrono::{Datelike, Local, TimeZone, Timelike};
use std::sync::Arc;
use string_interner::DefaultSymbol as Symbol;

pub fn setup_datetime_class(interner_ref: &SharedInterner) -> (Symbol, SharedMut<ClassDefinition>) {
    let name_sym = interner_ref.write(|i| i.get_or_intern("ДатаВремя"));
    let mut class_def = ClassDefinition::new(name_sym, Span::default());

    let ms_sym = interner_ref.write(|i| i.get_or_intern("_мс"));

    class_def.set_constructor(BuiltinFn(Arc::new(move |interp, args, span| {
        let instance = match args.get(0) {
            Some(Value::Object(inst)) => inst,
            _ => return Err(RuntimeError::TypeError(ErrorData::new(span, "Ошибка инициализации self".into()))),
        };

        let ms = if let Some(val) = args.get(1) {
            val.as_i64().ok_or_else(|| RuntimeError::TypeError(ErrorData::new(span, "Аргумент должен быть числом".into())))?
        } else {
            Local::now().timestamp_millis()
        };

        instance.write(|i| i.field_values.insert(ms_sym, Value::Number(ms)));

        Ok(Value::Empty)
    })));

    // --- Вспомогательная функция: извлечь мс из self ---
    let get_ms = move |args: &Vec<Value>| -> Result<i64, RuntimeError> {
        if let Some(Value::Object(inst)) = args.get(0) {
            inst.read(|i| {
                i.field_values
                    .get(&ms_sym)
                    .and_then(|v| v.as_i64()) // Твой новый хелпер
                    .ok_or_else(|| {
                        RuntimeError::InvalidOperation(ErrorData::new(
                            Span::default(),
                            "Объект ДатаВремя поврежден".into(),
                        ))
                    })
            })
        } else {
            Err(RuntimeError::InvalidOperation(ErrorData::new(
                Span::default(),
                "Метод должен вызываться у объекта".into(),
            )))
        }
    };

    // --- Методы получения компонентов (год, месяц, день, час, минута, секунда) ---
    let components = ["год", "месяц", "день", "час", "минута", "секунда"];

    for name in components {
        let method_name = name.to_string();
        class_def.add_method(
            interner_ref.write(|i| i.get_or_intern(&method_name)),
            Visibility::Public,
            false,
            BuiltinFn(Arc::new(move |_, args, _| {
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
            })),
        );
    }

    let units = [
        ("секунд", 1_000),
        ("минут", 60_000),
        ("часов", 3_600_000),
        ("дней", 86_400_000),
        ("месяцев", 2_592_000_000),
        ("лет", 31_536_000_000),
    ];

    for (name, ms_unit) in units {
        // --- Метод: ДОБАВИТЬ ---
        let add_name = format!("добавить_{}", name);
        class_def.add_method(
            interner_ref.write(|i| i.get_or_intern(&add_name)),
            Visibility::Public,
            false,
            BuiltinFn(Arc::new(move |_, args, span| {
                let current_ms = get_ms(&args)?;
                let val = args.get(1).and_then(|v| v.as_i64()).unwrap_or(0);

                let new_ms = current_ms + (val * ms_unit);

                if let Some(Value::Object(inst)) = args.get(0) {
                    inst.write(|i| i.field_values.insert(ms_sym, Value::Number(new_ms)));
                }
                Ok(args[0].clone())
            })),
        );

        // --- Метод: ВЫЧЕСТЬ ---
        let sub_name = format!("вычесть_{}", name);
        class_def.add_method(
            interner_ref.write(|i| i.get_or_intern(&sub_name)),
            Visibility::Public,
            false,
            BuiltinFn(Arc::new(move |_, args, span| {
                let current_ms = get_ms(&args)?;
                let val = args.get(1).and_then(|v| v.as_i64()).unwrap_or(0);

                let new_ms = current_ms - (val * ms_unit);

                if let Some(Value::Object(inst)) = args.get(0) {
                    inst.write(|i| i.field_values.insert(ms_sym, Value::Number(new_ms)));
                }
                Ok(args[0].clone())
            })),
        );
    }

    // --- Метод: .сейчас() (стандартный вывод) ---
    class_def.add_method(
        interner_ref.write(|i| i.get_or_intern("сейчас")),
        Visibility::Public,
        false, // Вызывается у инстанса
        BuiltinFn(Arc::new(move |_, args, _| {
            let now = Local::now();

            let pattern = match args.get(1) {
                Some(Value::Text(t)) => t.as_str(),
                _ => "%d.%m.%Y %H:%M:%S",
            };

            let formatted = now.format(pattern).to_string();
            Ok(Value::Text(formatted))
        })),
    );

    // --- Метод: .формат(шаблон) ---
    class_def.add_method(
        interner_ref.write(|i| i.get_or_intern("формат")),
        Visibility::Public,
        false,
        BuiltinFn(Arc::new(move |_, args, _| {
            let ms = get_ms(&args)?;
            let dt = Local.timestamp_millis_opt(ms).unwrap();
            let pattern = args
                .get(1)
                .and_then(|v| v.as_str())
                .map(|s| s.as_str())
                .unwrap_or("%d.%m.%Y %H:%M:%S");
            Ok(Value::Text(dt.format(pattern).to_string()))
        })),
    );

    (name_sym, SharedMut::new(class_def))
}
