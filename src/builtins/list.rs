use crate::ast::prelude::{ClassDefinition, ErrorData, Span, Visibility};
use crate::interpreter::prelude::{BuiltinFn, RuntimeError, SharedInterner, Value};
use crate::shared::SharedMut;
use crate::traits::prelude::CoreOperations;
use std::sync::Arc;
use string_interner::DefaultSymbol as Symbol;

pub fn setup_list_class(interner: &SharedInterner) -> (Symbol, SharedMut<ClassDefinition>) {
    let name = interner.write(|i| i.get_or_intern("Список"));

    let mut class_def = ClassDefinition::new(name, Span::default());

    class_def.set_constructor(BuiltinFn(Arc::new(|_interp, args, _span| {
        // args[0] - это временный ClassInstance (this)
        // args[1..] - это элементы: новый Список(1, 2, 3)
        if let Some(Value::Object(instance)) = args.get(0) {
            let items = args[1..].to_vec();
            let internal_list = Value::List(SharedMut::new(items));

            let data_sym = _interp.intern_string("__data");
            instance.write(|i| i.field_values.insert(data_sym, internal_list));
        }
        Ok(Value::Empty)
    })));

    // append(value) - Добавить в конец
    class_def.add_method(
        interner.write(|i| i.get_or_intern("добавить")),
        Visibility::Public,
        false,
        BuiltinFn(Arc::new(|_interp, args, span| {
            if let (Some(Value::List(list)), Some(val)) = (args.get(0), args.get(1)) {
                list.write(|i| i.push(val.clone()));
                Ok(Value::Empty)
            } else {
                Err(RuntimeError::TypeError(ErrorData::new(
                    span,
                    "Использование: list.append(value)".into(),
                )))
            }
        })),
    );

    // set(index: Number, value: Any) -> Empty
    class_def.add_method(
        interner.write(|i| i.get_or_intern("задать")),
        Visibility::Public,
        false,
        BuiltinFn(Arc::new(|_interp, args, span| {
            if let (Some(Value::List(list)), Some(Value::Number(idx)), Some(new_val)) =
                (args.get(0), args.get(1), args.get(2))
            {
                list.write(|vec| {
                    let i = *idx as usize;
                    if i < vec.len() {
                        vec[i] = new_val.clone();
                        Ok(Value::Empty)
                    } else {
                        Err(RuntimeError::InvalidOperation(ErrorData::new(
                            span,
                            "Индекс вне границ списка".into(),
                        )))
                    }
                })
            } else {
                Err(RuntimeError::TypeError(ErrorData::new(
                    span,
                    "Использование: list.set(number, value)".into(),
                )))
            }
        })),
    );

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

    // pop(index?) - Удалить и вернуть элемент (последний или по индексу)
    class_def.add_method(
        interner.write(|i| i.get_or_intern("удалить")),
        Visibility::Public,
        false,
        BuiltinFn(Arc::new(|_interp, args, span| {
            if let Some(Value::List(list)) = args.get(0) {
                list.write(|vec| {
                    if vec.is_empty() {
                        return Err(RuntimeError::InvalidOperation(ErrorData::new(
                            span,
                            "pop у пустого списка".into(),
                        )));
                    }

                    let val = if let Some(Value::Number(idx)) = args.get(1) {
                        if *idx < 0 || *idx >= vec.len() as i64 {
                            return Err(RuntimeError::InvalidOperation(ErrorData::new(
                                span,
                                "Индекс вне границ".into(),
                            )));
                        }
                        vec.remove(*idx as usize)
                    } else {
                        vec.pop().unwrap()
                    };

                    Ok(val)
                })
            } else {
                Err(RuntimeError::TypeError(ErrorData::new(
                    span,
                    "Ожидался List".into(),
                )))
            }
        })),
    );
    // clear() - Очистить список
    class_def.add_method(
        interner.write(|i| i.get_or_intern("отчистить")),
        Visibility::Public,
        false,
        BuiltinFn(Arc::new(|_interp, args, span| {
            if let Some(Value::List(list)) = args.get(0) {
                list.write(|i| i.clear());
                Ok(Value::Empty)
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
                let vec = list.read(|i| {
                    i.iter()
                        .map(|v| v.to_string())
                        .collect::<Vec<_>>()
                        .join(sep)
                });
                Ok(Value::Text(vec))
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
            if let (Some(Value::List(list)), Some(Value::Number(idx))) = (args.get(0), args.get(1))
            {
                let vec = list.read(|i| {
                    i.get(*idx as usize).cloned().ok_or_else(|| {
                        RuntimeError::InvalidOperation(ErrorData::new(
                            span,
                            "Индекс вне границ".into(),
                        ))
                    })
                });
                vec
            } else {
                Err(RuntimeError::TypeError(ErrorData::new(
                    span,
                    "Использование: list.get(number)".into(),
                )))
            }
        })),
    );

    (name, SharedMut::new(class_def))
}
