use crate::ast::prelude::{ClassDefinition, ErrorData, Span};
use crate::interpreter::prelude::{
    CallArgListExt, Interpreter, RuntimeError, SharedInterner, Value,
};
use crate::shared::SharedMut;
use crate::{define_builtin, define_constructor, define_method};
use std::collections::HashMap;
use string_interner::DefaultSymbol as Symbol;

pub fn setup_dict_class(interner: &SharedInterner) -> (Symbol, SharedMut<ClassDefinition>) {
    let name = interner.write(|i| i.get_or_intern("Словарь"));

    let mut class_def = ClassDefinition::new(name, Span::default());

    define_constructor!(class_def, (interp, args, span) {
        if let Some(Value::Object(instance)) = CallArgListExt::first_value(&args) {
            let internal_dict = Value::Dict(SharedMut::new(HashMap::new()));

            let data_sym = interp.interner.write(|i| i.get_or_intern("__data"));
            instance.write(|i| i.field_values.insert(data_sym, internal_dict));

            Ok(Value::Empty)
        } else {
            Err(RuntimeError::TypeError(ErrorData::new(
                span,
                "Ошибка конструктора словаря".into(),
            )))
        }
    });

    // 1. set(key: Text, value: Any) -> Empty
    define_method!(class_def, interner, "задать" => (_, args, span) {
        if let (Some(Value::Dict(dict)), Some(Value::Text(key)), Some(val)) = (
            CallArgListExt::first_value(&args),
            CallArgListExt::get_value(&args, 1),
            CallArgListExt::get_value(&args, 2),
        ) {
            dict.write(|i| i.insert(key.clone(), val.clone()));
            Ok(Value::Empty)
        } else {
            Err(RuntimeError::TypeError(ErrorData::new(
                span,
                "Использование: dict.set(string, value)".into(),
            )))
        }
    });

    // 2. get(key: Text, default?: Any) -> Any
    define_method!(class_def, interner, "получить" => (_, args, span) {
        if let (Some(Value::Dict(dict)), Some(Value::Text(key))) = (
            CallArgListExt::first_value(&args),
            CallArgListExt::get_value(&args, 1),
        ) {
            let result = dict.read(|d| {
                d.get(key)
                    .cloned()
                    .unwrap_or_else(|| {
                        CallArgListExt::get_value(&args, 2)
                            .cloned()
                            .unwrap_or(Value::Empty)
                    })
            });

            Ok(result)
        } else {
            Err(RuntimeError::TypeError(ErrorData::new(
                span,
                "Использование: dict.get(string, default?)".into(),
            )))
        }
    });

    // 3. has(key: Text) -> Boolean
    define_method!(class_def, interner, "имеет" => (_, args, span) {
        if let (Some(Value::Dict(dict)), Some(Value::Text(key))) = (
            CallArgListExt::first_value(&args),
            CallArgListExt::get_value(&args, 1),
        ) {
            Ok(Value::Boolean(dict.read(|i| i.contains_key(key))))
        } else {
            Err(RuntimeError::TypeError(ErrorData::new(
                span,
                "Использование: dict.has(string)".into(),
            )))
        }
    });

    // 4. keys() -> List<Text>
    define_method!(class_def, interner, "ключи" => (_, args, span) {
        if let Some(Value::Dict(dict)) = CallArgListExt::first_value(&args) {
            let keys: Vec<Value> =
                dict.read(|i| i.keys().map(|k| Value::Text(k.clone())).collect());
            Ok(Value::List(SharedMut::new(keys)))
        } else {
            Err(RuntimeError::TypeError(ErrorData::new(
                span,
                "Ожидался словарь".into(),
            )))
        }
    });

    // values() -> List<Any>
    define_method!(class_def, interner, "значения" => (_, args, span) {
        if let Some(Value::Dict(dict)) = CallArgListExt::first_value(&args) {
            let values: Vec<Value> = dict.read(|i| i.values().cloned().collect());
            Ok(Value::List(SharedMut::new(values)))
        } else {
            Err(RuntimeError::TypeError(ErrorData::new(
                span,
                "Ожидался словарь".into(),
            )))
        }
    });

    // 5. remove(key: Text) -> Any
    define_method!(class_def, interner, "удалить" => (_, args, span) {
        if let (Some(Value::Dict(dict)), Some(Value::Text(key))) = (
            CallArgListExt::first_value(&args),
            CallArgListExt::get_value(&args, 1),
        ) {
            Ok(dict.write(|i| i.remove(key)).unwrap_or(Value::Empty))
        } else {
            Err(RuntimeError::TypeError(ErrorData::new(
                span,
                "Использование: dict.remove(string)".into(),
            )))
        }
    });

    // 6. len() -> Number
    define_method!(class_def, interner, "длина" => (_, args, span) {
        if let Some(Value::Dict(dict)) = CallArgListExt::first_value(&args) {
            Ok(Value::Number(dict.read(|i| i.len() as i64)))
        } else {
            Err(RuntimeError::TypeError(ErrorData::new(
                span,
                "Ожидался словарь".into(),
            )))
        }
    });

    (name, SharedMut::new(class_def))
}

pub fn setup_dict_func(interpreter: &mut Interpreter, interner: &SharedInterner) {
    define_builtin!(interpreter, interner, "словарь" => (_, arguments, span) {
        if arguments.len() % 2 != 0 {
            return Err(RuntimeError::InvalidOperation(ErrorData::new(
                span,
                "Функция 'словарь' ожидает четное количество аргументов (пары ключ-значение)"
                    .to_string(),
            )));
        }

        let mut dict = HashMap::new();
        for i in (0..arguments.len()).step_by(2) {
            let key = match &arguments[i].value {
                Value::Text(s) => s.clone(),
                v => v.to_string(),
            };
            let value = arguments[i + 1].value.clone();
            dict.insert(key, value);
        }

        Ok(Value::Dict(SharedMut::new(dict)))
    });
}
