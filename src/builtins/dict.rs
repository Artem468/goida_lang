use crate::ast::prelude::{ClassDefinition, ErrorData, Span, Visibility};
use crate::interpreter::prelude::{BuiltinFn, Interpreter, RuntimeError, SharedInterner, Value};
use crate::shared::SharedMut;
use std::collections::HashMap;
use std::sync::Arc;
use string_interner::DefaultSymbol as Symbol;

pub fn setup_dict_class(interner: &SharedInterner) -> (Symbol, SharedMut<ClassDefinition>) {
    let name = interner.write(|i| i.get_or_intern("Словарь"));

    let mut class_def = ClassDefinition::new(name, Span::default());

    class_def.set_constructor(BuiltinFn(Arc::new(|_interp, args, span| {
        if let Some(Value::Object(instance)) = args.get(0) {
            let internal_dict = Value::Dict(SharedMut::new(HashMap::new()));

            let data_sym = _interp.interner.write(|i| i.get_or_intern("__data"));
            instance.write(|i| i.field_values.insert(data_sym, internal_dict));
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
        interner.write(|i| i.get_or_intern("задать")),
        Visibility::Public,
        false,
        BuiltinFn(Arc::new(|_interp, args, span| {
            if let (Some(Value::Dict(dict)), Some(Value::Text(key)), Some(val)) =
                (args.get(0), args.get(1), args.get(2))
            {
                dict.write(|i| i.insert(key.clone(), val.clone()));
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
        interner.write(|i| i.get_or_intern("получить")),
        Visibility::Public,
        false,
        BuiltinFn(Arc::new(|_interp, args, span| {
            if let (Some(Value::Dict(dict)), Some(Value::Text(key))) = (args.get(0), args.get(1)) {
                let result = dict.read(|d| {
                    d.get(key)
                        .cloned() // Клонируем значение из словаря
                        .unwrap_or_else(|| args.get(2).cloned().unwrap_or(Value::Empty))
                });

                Ok(result)
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
        interner.write(|i| i.get_or_intern("имеет")),
        Visibility::Public,
        false,
        BuiltinFn(Arc::new(|_interp, args, span| {
            if let (Some(Value::Dict(dict)), Some(Value::Text(key))) = (args.get(0), args.get(1)) {
                Ok(Value::Boolean(dict.read(|i| i.contains_key(key))))
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
        interner.write(|i| i.get_or_intern("ключи")),
        Visibility::Public,
        false,
        BuiltinFn(Arc::new(|_interp, args, span| {
            if let Some(Value::Dict(dict)) = args.get(0) {
                let keys: Vec<Value> =
                    dict.read(|i| i.keys().map(|k| Value::Text(k.clone())).collect());
                Ok(Value::List(SharedMut::new(keys)))
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
        interner.write(|i| i.get_or_intern("значения")),
        Visibility::Public,
        false,
        BuiltinFn(Arc::new(|_interp, args, span| {
            if let Some(Value::Dict(dict)) = args.get(0) {
                let values: Vec<Value> = dict.read(|i| i.values().cloned().collect());
                Ok(Value::List(SharedMut::new(values)))
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
        interner.write(|i| i.get_or_intern("удалить")),
        Visibility::Public,
        false,
        BuiltinFn(Arc::new(|_interp, args, span| {
            if let (Some(Value::Dict(dict)), Some(Value::Text(key))) = (args.get(0), args.get(1)) {
                Ok(dict.write(|i| i.remove(key)).unwrap_or(Value::Empty))
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
        interner.write(|i| i.get_or_intern("длина")),
        Visibility::Public,
        false,
        BuiltinFn(Arc::new(|_interp, args, span| {
            if let Some(Value::Dict(dict)) = args.get(0) {
                Ok(Value::Number(dict.read(|i| i.len() as i64)))
            } else {
                Err(RuntimeError::TypeError(ErrorData::new(
                    span,
                    "Ожидался Dict".into(),
                )))
            }
        })),
    );

    (name, SharedMut::new(class_def))
}

pub fn setup_dict_func(interpreter: &mut Interpreter, interner: &SharedInterner) {
    interpreter.builtins.insert(
        interner.write(|i| i.get_or_intern("словарь")),
        BuiltinFn(Arc::new(move |_interpreter, arguments, span| {
            if arguments.len() % 2 != 0 {
                return Err(RuntimeError::InvalidOperation(ErrorData::new(
                    span,
                    "Функция 'словарь' ожидает четное количество аргументов (пары ключ-значение)".to_string()
                )));
            }

            let mut dict = HashMap::new();
            for i in (0..arguments.len()).step_by(2) {
                let key = match &arguments[i] {
                    Value::Text(s) => s.clone(),
                    v => v.to_string(),
                };
                let value = arguments[i + 1].clone();
                dict.insert(key, value);
            }

            Ok(Value::Dict(SharedMut::new(dict)))
        })),
    );
}