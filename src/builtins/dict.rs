use crate::ast::prelude::{ClassDefinition, ErrorData, Span, Visibility};
use crate::interpreter::prelude::{BuiltinFn, RuntimeError, SharedInterner, Value};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use string_interner::DefaultSymbol as Symbol;

pub fn setup_dict_class(interner: &SharedInterner) -> (Symbol, Arc<RwLock<ClassDefinition>>) {
    let name = interner
        .write()
        .expect("interner lock poisoned")
        .get_or_intern("Словарь");

    let mut class_def = ClassDefinition::new(name, Span::default());

    class_def.set_constructor(BuiltinFn(Arc::new(|_interp, args, span| {
        if let Some(Value::Object(instance)) = args.get(0) {
            let internal_dict = Value::Dict(Arc::new(RwLock::new(HashMap::new())));

            let data_sym = _interp.interner.write().unwrap().get_or_intern("__data");
            instance
                .write()
                .map_err(|_| {
                    RuntimeError::Panic(ErrorData::new(
                        Span::default(),
                        "Сбой блокировки в реализации словаря".into(),
                    ))
                })?
                .field_values
                .insert(data_sym, internal_dict);
            Ok(Value::Empty)
        } else {
            Err(RuntimeError::TypeError(ErrorData::new(
                span,
                "Ошибка конструктора Dict".into(),
            )))
        }
    })));

    // 1. set(key: Text, value: Any) -> Empty
    class_def.add_method(
        interner
            .write()
            .expect("interner lock poisoned")
            .get_or_intern("задать"),
        Visibility::Public,
        false,
        BuiltinFn(Arc::new(|_interp, args, span| {
            if let (Some(Value::Dict(dict)), Some(Value::Text(key)), Some(val)) =
                (args.get(0), args.get(1), args.get(2))
            {
                dict.write()
                    .map_err(|_| {
                        RuntimeError::Panic(ErrorData::new(
                            Span::default(),
                            "Сбой блокировки в реализации словаря".into(),
                        ))
                    })?
                    .insert(key.clone(), val.clone());
                Ok(Value::Empty)
            } else {
                Err(RuntimeError::TypeError(ErrorData::new(
                    span,
                    "Использование: dict.set(string, value)".into(),
                )))
            }
        })),
    );

    // 2. get(key: Text, default?: Any) -> Any
    class_def.add_method(
        interner
            .write()
            .expect("interner lock poisoned")
            .get_or_intern("получить"),
        Visibility::Public,
        false,
        BuiltinFn(Arc::new(|_interp, args, span| {
            if let (Some(Value::Dict(dict)), Some(Value::Text(key))) = (args.get(0), args.get(1)) {
                let dict = dict.read().map_err(|_| {
                    RuntimeError::Panic(ErrorData::new(
                        Span::default(),
                        "Сбой блокировки в реализации словаря".into(),
                    ))
                })?;
                if let Some(val) = dict.get(key) {
                    Ok(val.clone())
                } else {
                    Ok(args.get(2).cloned().unwrap_or(Value::Empty))
                }
            } else {
                Err(RuntimeError::TypeError(ErrorData::new(
                    span,
                    "Использование: dict.get(string, default?)".into(),
                )))
            }
        })),
    );

    // 3. has(key: Text) -> Boolean
    class_def.add_method(
        interner
            .write()
            .expect("interner lock poisoned")
            .get_or_intern("имеет"),
        Visibility::Public,
        false,
        BuiltinFn(Arc::new(|_interp, args, span| {
            if let (Some(Value::Dict(dict)), Some(Value::Text(key))) = (args.get(0), args.get(1)) {
                Ok(Value::Boolean(
                    dict.read()
                        .map_err(|_| {
                            RuntimeError::Panic(ErrorData::new(
                                Span::default(),
                                "Сбой блокировки в реализации словаря".into(),
                            ))
                        })?
                        .contains_key(key),
                ))
            } else {
                Err(RuntimeError::TypeError(ErrorData::new(
                    span,
                    "Использование: dict.has(string)".into(),
                )))
            }
        })),
    );

    // 4. keys() -> List<Text>
    class_def.add_method(
        interner
            .write()
            .expect("interner lock poisoned")
            .get_or_intern("ключи"),
        Visibility::Public,
        false,
        BuiltinFn(Arc::new(|_interp, args, span| {
            if let Some(Value::Dict(dict)) = args.get(0) {
                let keys: Vec<Value> = dict
                    .read()
                    .map_err(|_| {
                        RuntimeError::Panic(ErrorData::new(
                            Span::default(),
                            "Сбой блокировки в реализации словаря".into(),
                        ))
                    })?
                    .keys()
                    .map(|k| Value::Text(k.clone()))
                    .collect();
                Ok(Value::List(Arc::new(RwLock::new(keys))))
            } else {
                Err(RuntimeError::TypeError(ErrorData::new(
                    span,
                    "Ожидался Dict".into(),
                )))
            }
        })),
    );

    // values() -> List<Any>
    class_def.add_method(
        interner
            .write()
            .expect("interner lock poisoned")
            .get_or_intern("значения"),
        Visibility::Public,
        false,
        BuiltinFn(Arc::new(|_interp, args, span| {
            if let Some(Value::Dict(dict)) = args.get(0) {
                let values: Vec<Value> = dict
                    .read()
                    .map_err(|_| {
                        RuntimeError::Panic(ErrorData::new(
                            Span::default(),
                            "Сбой блокировки в реализации словаря".into(),
                        ))
                    })?
                    .values()
                    .cloned()
                    .collect();
                Ok(Value::List(Arc::new(RwLock::new(values))))
            } else {
                Err(RuntimeError::TypeError(ErrorData::new(
                    span,
                    "Ожидался Dict".into(),
                )))
            }
        })),
    );

    // 5. remove(key: Text) -> Any
    class_def.add_method(
        interner
            .write()
            .expect("interner lock poisoned")
            .get_or_intern("удалить"),
        Visibility::Public,
        false,
        BuiltinFn(Arc::new(|_interp, args, span| {
            if let (Some(Value::Dict(dict)), Some(Value::Text(key))) = (args.get(0), args.get(1)) {
                Ok(dict
                    .write()
                    .map_err(|_| {
                        RuntimeError::Panic(ErrorData::new(
                            Span::default(),
                            "Сбой блокировки в реализации словаря".into(),
                        ))
                    })?
                    .remove(key)
                    .unwrap_or(Value::Empty))
            } else {
                Err(RuntimeError::TypeError(ErrorData::new(
                    span,
                    "Использование: dict.remove(string)".into(),
                )))
            }
        })),
    );

    // 6. len() -> Number
    class_def.add_method(
        interner
            .write()
            .expect("interner lock poisoned")
            .get_or_intern("длина"),
        Visibility::Public,
        false,
        BuiltinFn(Arc::new(|_interp, args, span| {
            if let Some(Value::Dict(dict)) = args.get(0) {
                Ok(Value::Number(
                    dict.read()
                        .map_err(|_| {
                            RuntimeError::Panic(ErrorData::new(
                                Span::default(),
                                "Сбой блокировки в реализации словаря".into(),
                            ))
                        })?
                        .len() as i64,
                ))
            } else {
                Err(RuntimeError::TypeError(ErrorData::new(
                    span,
                    "Ожидался Dict".into(),
                )))
            }
        })),
    );

    (name, Arc::new(RwLock::new(class_def)))
}
