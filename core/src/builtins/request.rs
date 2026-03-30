use crate::ast::prelude::{ClassDefinition, ErrorData, Span, Visibility};
use crate::builtins::response::build_response_object;
use crate::interpreter::prelude::{BuiltinFn, CallArgListExt, RuntimeError, SharedInterner, Value};
use crate::shared::SharedMut;
use crate::traits::json::JsonParsable;
use std::collections::HashMap;
use std::sync::Arc;
use string_interner::DefaultSymbol as Symbol;

pub fn setup_request_class(interner_ref: &SharedInterner) -> (Symbol, SharedMut<ClassDefinition>) {
    let (name_sym, url_sym, method_sym, headers_sym, body_sym) = interner_ref.write(|i| {
        (
            i.get_or_intern("Запрос"),
            i.get_or_intern("урл"),
            i.get_or_intern("метод"),
            i.get_or_intern("заголовки"),
            i.get_or_intern("тело"),
        )
    });

    let mut class_def = ClassDefinition::new(name_sym, Span::default());

    class_def.set_constructor(BuiltinFn(Arc::new(move |_interp, args, span| {
        let inst = CallArgListExt::first_value(&args)
            .unwrap()
            .as_object(span)?;
        let url = CallArgListExt::get_value(&args, 1)
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                RuntimeError::TypeError(ErrorData::new(span, "Ожидалась строка".into()))
            })?;

        inst.write(|i| {
            i.field_values.insert(url_sym, Value::Text(url.clone()));
            i.field_values.insert(method_sym, Value::Text("GET".into()));
            i.field_values
                .insert(headers_sym, Value::Dict(SharedMut::new(HashMap::new())));
            i.field_values.insert(body_sym, Value::Empty);
        });
        Ok(Value::Empty)
    })));

    class_def.add_method(
        method_sym,
        Visibility::Public,
        false,
        BuiltinFn(Arc::new(move |_, args, span| {
            let inst = CallArgListExt::first_value(&args)
                .unwrap()
                .as_object(span)?;
            let method = CallArgListExt::get_value(&args, 1)
                .and_then(|v| v.as_str())
                .ok_or_else(|| {
                    RuntimeError::TypeError(ErrorData::new(span, "Ожидалась строка".into()))
                })?;

            inst.write(|i| {
                i.field_values
                    .insert(method_sym, Value::Text(method.clone()))
            });
            Ok(args[0].value.clone())
        })),
    );

    class_def.add_method(
        interner_ref.write(|i| i.get_or_intern("заголовок")),
        Visibility::Public,
        false,
        BuiltinFn(Arc::new(move |_interp, args, span| {
            let inst = CallArgListExt::first_value(&args)
                .unwrap()
                .as_object(span)?;
            let key = CallArgListExt::get_value(&args, 1)
                .and_then(|v| v.as_str())
                .ok_or_else(|| {
                    RuntimeError::TypeError(ErrorData::new(span, "Ожидалась строка ключа".into()))
                })?;
            let val = CallArgListExt::get_value(&args, 2)
                .cloned()
                .ok_or_else(|| {
                    RuntimeError::InvalidOperation(ErrorData::new(
                        span,
                        "Метод 'заголовок' ожидает 2 аргумента".into(),
                    ))
                })?;

            let headers_val = inst
                .read(|i| i.field_values.get(&headers_sym).cloned())
                .ok_or_else(|| {
                    RuntimeError::Panic(ErrorData::new(
                        span,
                        "Поле заголовков не инициализировано".into(),
                    ))
                })?;

            if let Value::Dict(headers) = headers_val {
                headers.write(|map| map.insert(key.clone(), val));
            }

            Ok(args[0].value.clone())
        })),
    );

    class_def.add_method(
        body_sym,
        Visibility::Public,
        false,
        BuiltinFn(Arc::new(move |_interp, args, span| {
            let inst = CallArgListExt::first_value(&args)
                .unwrap()
                .as_object(span)?;
            let value = CallArgListExt::get_value(&args, 1)
                .cloned()
                .ok_or_else(|| {
                    RuntimeError::InvalidOperation(ErrorData::new(
                        span,
                        "Метод 'тело' ожидает 1 аргумент".into(),
                    ))
                })?;

            inst.write(|i| i.field_values.insert(body_sym, value));
            Ok(args[0].value.clone())
        })),
    );

    class_def.add_method(
        interner_ref.write(|i| i.get_or_intern("json")),
        Visibility::Public,
        false,
        BuiltinFn(Arc::new(move |_interp, args, span| {
            let inst = CallArgListExt::first_value(&args)
                .unwrap()
                .as_object(span)?;
            let value = CallArgListExt::get_value(&args, 1)
                .cloned()
                .ok_or_else(|| {
                    RuntimeError::InvalidOperation(ErrorData::new(
                        span,
                        "Метод 'json' ожидает 1 аргумент".into(),
                    ))
                })?;

            let headers_val = inst
                .read(|i| i.field_values.get(&headers_sym).cloned())
                .ok_or_else(|| {
                    RuntimeError::Panic(ErrorData::new(
                        span,
                        "Поле заголовков не инициализировано".into(),
                    ))
                })?;

            if let Value::Dict(headers) = headers_val {
                headers.write(|map| {
                    map.insert(
                        "Content-Type".to_string(),
                        Value::Text("application/json".to_string()),
                    );
                });
            }

            inst.write(|i| i.field_values.insert(body_sym, value));
            Ok(args[0].value.clone())
        })),
    );

    class_def.add_method(
        interner_ref.write(|i| i.get_or_intern("отправить")),
        Visibility::Public,
        false,
        BuiltinFn(Arc::new(move |interp, args, span| {
            let inst = CallArgListExt::first_value(&args)
                .unwrap()
                .as_object(span)?;

            let (url, method, headers, body) = inst.read(|i| {
                let url = i
                    .field_values
                    .get(&url_sym)
                    .and_then(|v| v.as_str())
                    .cloned()
                    .unwrap_or_default();
                let method = i
                    .field_values
                    .get(&method_sym)
                    .and_then(|v| v.as_str())
                    .cloned()
                    .unwrap_or_else(|| "GET".into());
                let headers = i.field_values.get(&headers_sym).cloned().unwrap();
                let body = i
                    .field_values
                    .get(&body_sym)
                    .cloned()
                    .unwrap_or(Value::Empty);
                (url, method, headers, body)
            });

            let mut builder = ureq::http::Request::builder()
                .method(method.to_uppercase().as_str())
                .uri(&url);

            let mut header_pairs: Vec<(String, String)> = Vec::new();

            if let Value::Dict(headers) = headers {
                headers.read(|map| {
                    for (key, value) in map {
                        header_pairs.push((key.clone(), value.to_string()));
                    }
                });
            }

            for (key, value) in header_pairs {
                builder = builder.header(key, value);
            }

            let body_bytes = match body {
                Value::Text(text) => text.into_bytes(),
                Value::Empty => Vec::new(),
                value => {
                    let json = value.to_json().map_err(|error| {
                        RuntimeError::InvalidOperation(ErrorData::new(
                            span,
                            format!("Ошибка сериализации тела запроса: {}", error),
                        ))
                    })?;

                    serde_json::to_vec(&json).map_err(|error| {
                        RuntimeError::InvalidOperation(ErrorData::new(
                            span,
                            format!("Ошибка сериализации тела запроса: {}", error),
                        ))
                    })?
                }
            };

            let request = builder.body(body_bytes).map_err(|e| {
                RuntimeError::InvalidOperation(ErrorData::new(
                    span,
                    format!("Ошибка сборки запроса: {}", e),
                ))
            })?;

            let response = ureq::run(request).map_err(|e| {
                RuntimeError::IOError(ErrorData::new(span, format!("Ошибка сети: {}", e)))
            })?;

            build_response_object(interp, response, span)
        })),
    );

    (name_sym, SharedMut::new(class_def))
}
