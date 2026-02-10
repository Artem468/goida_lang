use crate::ast::prelude::{ClassDefinition, ErrorData, Span, Visibility};
use crate::builtins::response::build_response_object;
use crate::interpreter::prelude::{BuiltinFn, RuntimeError, SharedInterner, Value};
use crate::shared::SharedMut;
use std::collections::HashMap;
use std::sync::Arc;
use string_interner::DefaultSymbol as Symbol;

pub fn setup_request_class(interner_ref: &SharedInterner) -> (Symbol, SharedMut<ClassDefinition>) {
    let (name_sym, url_sym, method_sym, headers_sym, body_sym) = interner_ref.write(|i| (
        i.get_or_intern("Запрос"),
        i.get_or_intern("урл"),
        i.get_or_intern("метод"),
        i.get_or_intern("заголовки"),
        i.get_or_intern("тело"),
    ));

    let mut class_def = ClassDefinition::new(name_sym, Span::default());

    class_def.set_constructor(BuiltinFn(Arc::new(move |_interp, args, span| {
        let inst = args.get(0).unwrap().as_object(span)?;
        let url = args.get(1)
            .and_then(|v| v.as_str())
            .ok_or_else(|| RuntimeError::TypeError(ErrorData::new(span, "Ожидалась строка".into())))?;

        inst.write(|i| {
            i.field_values.insert(url_sym, Value::Text(url.clone()));
            i.field_values.insert(method_sym, Value::Text("GET".into()));
            i.field_values.insert(headers_sym, Value::Dict(SharedMut::new(HashMap::new())));
            i.field_values.insert(body_sym, Value::Empty);
        });
        Ok(Value::Empty)
    })));

    // Метод: .метод("POST")
    class_def.add_method(
        method_sym, // Используем уже готовый символ
        Visibility::Public,
        false,
        BuiltinFn(Arc::new(move |_, args, span| {
            let inst = args.get(0).unwrap().as_object(span)?;
            let m = args.get(1)
                .and_then(|v| v.as_str())
                .ok_or_else(|| RuntimeError::TypeError(ErrorData::new(span, "Ожидалась строка".into())))?;

            // Исправлено: записываем в поле 'метод', а не в имя класса
            inst.write(|i| i.field_values.insert(method_sym, Value::Text(m.clone())));
            Ok(args[0].clone())
        })),
    );

    // Метод: .заголовок("Key", "Value")
    class_def.add_method(
        interner_ref.write(|i| i.get_or_intern("заголовок")),
        Visibility::Public,
        false,
        BuiltinFn(Arc::new(move |_interp, args, span| {
            let inst = args.get(0).unwrap().as_object(span)?;
            let key = args.get(1)
                .and_then(|v| v.as_str())
                .ok_or_else(|| RuntimeError::TypeError(ErrorData::new(span, "Ожидалась строка ключа".into())))?;
            let val = args.get(2).unwrap().clone();

            let headers_val = inst.read(|i| i.field_values.get(&headers_sym).cloned())
                .ok_or_else(|| RuntimeError::Panic(ErrorData::new(span, "Поле заголовков не инициализировано".into())))?;

            if let Value::Dict(d) = headers_val {
                d.write(|map| map.insert(key.clone(), val));
            }
            Ok(args[0].clone())
        })),
    );

    // Метод: .отправить()
    class_def.add_method(
        interner_ref.write(|i| i.get_or_intern("отправить")),
        Visibility::Public,
        false,
        BuiltinFn(Arc::new(move |interp, args, span| {
            let inst = args.get(0).unwrap().as_object(span)?;

            // Извлекаем данные, используя захваченные символы
            let (url, method, headers, body) = inst.read(|i| {
                let u = i.field_values.get(&url_sym).and_then(|v| v.as_str()).cloned().unwrap_or_default();
                let m = i.field_values.get(&method_sym).and_then(|v| v.as_str()).cloned().unwrap_or_else(|| "GET".into());
                let h = i.field_values.get(&headers_sym).cloned().unwrap();
                let b = i.field_values.get(&body_sym).cloned().unwrap_or(Value::Empty);
                (u, m, h, b)
            });

            let mut builder = ureq::http::Request::builder()
                .method(method.to_uppercase().as_str())
                .uri(&url);

            let mut header_pairs: Vec<(String, String)> = Vec::new();

            if let Value::Dict(d) = headers {
                d.read(|map| {
                    for (k, v) in map {
                        header_pairs.push((k.clone(), v.to_string()));
                    }
                });
            }

            for (k, v) in header_pairs {
                builder = builder.header(k, v);
            }

            let body_bytes = match body {
                Value::Text(s) => s.into_bytes(),
                Value::Empty => Vec::new(),
                _ => body.to_string().into_bytes(),
            };

            // В ureq 3.x сборка и выполнение
            let request = builder.body(body_bytes).map_err(|e| {
                RuntimeError::InvalidOperation(ErrorData::new(span, format!("Ошибка сборки запроса: {}", e)))
            })?;

            let response = ureq::run(request).map_err(|e| {
                RuntimeError::IOError(ErrorData::new(span, format!("Ошибка сети: {}", e)))
            })?;

            build_response_object(interp, response, span)
        })),
    );

    (name_sym, SharedMut::new(class_def))
}
