use crate::ast::prelude::{ClassDefinition, ErrorData, Span, Visibility};
use crate::interpreter::prelude::{BuiltinFn, Interpreter, RuntimeError, SharedInterner, Value};
use crate::shared::SharedMut;
use crate::traits::json::JsonParsable; // Твой новый трейт
use std::collections::HashMap;
use std::sync::Arc;
use string_interner::DefaultSymbol as Symbol;
use ureq::Body;

pub fn setup_response_class(interner_ref: &SharedInterner) -> (Symbol, SharedMut<ClassDefinition>) {
    // 1. Подготавливаем символы (имена полей и класса)
    let (name_sym, status_sym, headers_sym, body_raw_sym) = interner_ref.write(|i| {
        (
            i.get_or_intern("Ответ"),
            i.get_or_intern("статус"),
            i.get_or_intern("заголовки"),
            i.get_or_intern("_тело_сырое"), // скрытое поле для внутреннего использования
        )
    });

    let mut class_def = ClassDefinition::new(name_sym, Span::default());

    // --- Метод: .код() -> Число (HTTP Status Code) ---
    class_def.add_method(
        interner_ref.write(|i| i.get_or_intern("код")),
        Visibility::Public,
        false,
        BuiltinFn(Arc::new(move |_, args, span| {
            let inst = args.get(0).unwrap().as_object(span)?;
            let val = inst
                .read(|i| i.field_values.get(&status_sym).cloned())
                .unwrap_or(Value::Number(0));
            Ok(val)
        })),
    );

    // --- Метод: .заголовки() -> Dict ---
    class_def.add_method(
        interner_ref.write(|i| i.get_or_intern("заголовки")),
        Visibility::Public,
        false,
        BuiltinFn(Arc::new(move |_, args, span| {
            let inst = args.get(0).unwrap().as_object(span)?;
            let val = inst
                .read(|i| i.field_values.get(&headers_sym).cloned())
                .unwrap_or_else(|| Value::Dict(SharedMut::new(HashMap::new())));
            Ok(val)
        })),
    );

    // --- Метод: .строка() -> Text (Тело ответа как текст) ---
    class_def.add_method(
        interner_ref.write(|i| i.get_or_intern("строка")),
        Visibility::Public,
        false,
        BuiltinFn(Arc::new(move |_, args, span| {
            let inst = args.get(0).unwrap().as_object(span)?;
            let val = inst
                .read(|i| i.field_values.get(&body_raw_sym).cloned())
                .unwrap_or_else(|| Value::Text("".into()));
            Ok(val)
        })),
    );

    // --- Метод: .словарь() -> Dict (Парсинг JSON тела) ---
    class_def.add_method(
        interner_ref.write(|i| i.get_or_intern("словарь")),
        Visibility::Public,
        false,
        BuiltinFn(Arc::new(move |_interp, args, span| {
            let inst = args.get(0).unwrap().as_object(span)?;

            // Извлекаем сырой текст тела
            let raw_text = inst
                .read(|i| {
                    i.field_values
                        .get(&body_raw_sym)
                        .and_then(|v| v.as_str()) // Твой хелпер as_str()
                        .cloned()
                })
                .unwrap_or_default();

            if raw_text.is_empty() {
                return Ok(Value::Dict(SharedMut::new(HashMap::new())));
            }

            // Используем serde_json + наш трейт JsonParsable
            let json_value: serde_json::Value = serde_json::from_str(&raw_text).map_err(|e| {
                RuntimeError::InvalidOperation(ErrorData::new(
                    span,
                    format!("Ошибка при разборе JSON: {}", e),
                ))
            })?;

            Ok(Value::from_json(json_value))
        })),
    );

    (name_sym, SharedMut::new(class_def))
}

pub fn build_response_object(
    interpreter: &Interpreter,
    mut resp: ureq::http::Response<Body>,
    span: Span,
) -> Result<Value, RuntimeError> {
    let status = resp.status().as_u16() as i64;

    // Собираем заголовки ответа
    let mut resp_headers = HashMap::new();
    for (name, value) in resp.headers() {
        let val_str = value.to_str().unwrap_or("").to_string();
        resp_headers.insert(name.to_string(), Value::Text(val_str));
    }

    // Читаем тело (ureq 3.2+)
    let body_text = resp.body_mut().read_to_string().map_err(|e| {
        RuntimeError::IOError(ErrorData::new(span, format!("Ошибка чтения тела: {}", e)))
    })?;

    // Ищем определение класса "Ответ"
    let resp_class_name = interpreter.interner.write(|i| i.get_or_intern("Ответ"));
    let class_def = match interpreter.std_classes.get(&resp_class_name) {
        Some(cls) => cls.clone(),
        None => {
            return Err(RuntimeError::InvalidOperation(ErrorData::new(
                span,
                "Не найден объект ответ".into(),
            )))
        }
    };

    // Создаем инстанс
    let inst = ClassDefinition::create_instance(class_def);
    let inst_ref = SharedMut::new(inst);
    // Получаем символы полей
    let s_sym = interpreter.interner.read(|i| i.get("статус").unwrap());
    let h_sym = interpreter.interner.read(|i| i.get("заголовки").unwrap());
    let b_sym = interpreter
        .interner
        .write(|i| i.get_or_intern("_тело_сырое"));

    inst_ref.write(|i| {
        i.field_values.insert(s_sym, Value::Number(status));
        i.field_values
            .insert(h_sym, Value::Dict(SharedMut::new(resp_headers)));
        i.field_values.insert(b_sym, Value::Text(body_text));
    });

    Ok(Value::Object(inst_ref))
}
