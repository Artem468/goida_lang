use crate::ast::prelude::{ClassDefinition, ErrorData, Span, Visibility};
use crate::interpreter::prelude::{BuiltinFn, RuntimeError, SharedInterner, Value};
use crate::shared::SharedMut;
use std::sync::Arc;
use string_interner::DefaultSymbol as Symbol;

pub fn setup_array_class(interner: &SharedInterner) -> (Symbol, SharedMut<ClassDefinition>) {
    let name = interner.write(|i| i.get_or_intern("Массив"));

    let mut class_def = ClassDefinition::new(name, Span::default());

    class_def.set_constructor(BuiltinFn(Arc::new(|_interp, args, _span| {
        if let Some(Value::Object(instance)) = args.get(0) {
            let items = args[1..].to_vec();
            let internal_array = Value::Array(Arc::new(items));

            let data_sym = _interp.interner.write(|i| i.get_or_intern("__data"));
            instance.write(|i| i.field_values.insert(data_sym, internal_array));
        }
        Ok(Value::Empty)
    })));

    // len() - Получить длину
    class_def.add_method(
        interner.write(|i| i.get_or_intern("длина")),
        Visibility::Public,
        false,
        BuiltinFn(Arc::new(|_interp, args, span| {
            if let Some(Value::List(list)) = args.get(0) {
                let length = list.read(|i| i.len());
                Ok(Value::Number(length as i64))
            } else {
                Err(RuntimeError::TypeError(ErrorData::new(
                    span,
                    "Ожидался List".into(),
                )))
            }
        })),
    );

    // join(separator) - Склеить в строку
    class_def.add_method(
        interner.write(|i| i.get_or_intern("объединить")),
        Visibility::Public,
        false,
        BuiltinFn(Arc::new(|_interp, args, span| {
            if let (Some(Value::List(list)), Some(Value::Text(sep))) = (args.get(0), args.get(1)) {
                let res = list.read(|vec| {
                    vec.iter()
                        .map(|v| v.to_string())
                        .collect::<Vec<_>>()
                        .join(sep)
                });

                Ok(Value::Text(res))
            } else {
                Err(RuntimeError::TypeError(ErrorData::new(
                    span,
                    "Использование: list.join(string)".into(),
                )))
            }
        })),
    );

    // get(index) - Безопасное получение (аналог list[i])
    class_def.add_method(
        interner.write(|i| i.get_or_intern("получить")),
        Visibility::Public,
        false,
        BuiltinFn(Arc::new(|_interp, args, span| {
            if let (Some(Value::Array(arr)), Some(idx)) = (args.get(0), args.get(1))
            {
                let i = idx.resolve_index(arr.len(), span)?;
                Ok(arr[i].clone())
            } else {
                Err(RuntimeError::TypeError(ErrorData::new(
                    span,
                    "Использование: array.get(number)".into(),
                )))
            }
        })),
    );

    (name, SharedMut::new(class_def))
}
