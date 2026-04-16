use crate::ast::prelude::{ClassDefinition, ErrorData, Span};
use crate::interpreter::prelude::{
    CallArgListExt, Interpreter, RuntimeError, SharedInterner, Value,
};
use crate::shared::SharedMut;
use crate::traits::prelude::CoreOperations;
use crate::{define_builtin, define_constructor, define_method};
use string_interner::DefaultSymbol as Symbol;

pub fn setup_list_class(interner: &SharedInterner) -> (Symbol, SharedMut<ClassDefinition>) {
    let name = interner.write(|i| i.get_or_intern("Список"));

    let mut class_def = ClassDefinition::new(name, Span::default());

    define_constructor!(class_def, (interp, args, _) {
        if let Some(Value::Object(instance)) = CallArgListExt::first_value(&args) {
            let items = args[1..].iter().map(|arg| arg.value.clone()).collect();
            let internal_list = Value::List(SharedMut::new(items));

            let data_sym = interp.intern_string("__data");
            instance.write(|i| i.field_values.insert(data_sym, internal_list));
        }
        Ok(Value::Empty)
    });

    // append(value) - Добавить в конец
    define_method!(class_def, interner, "добавить" => (_, args, span) {
        if let (Some(Value::List(list)), Some(val)) = (
            CallArgListExt::first_value(&args),
            CallArgListExt::get_value(&args, 1),
        ) {
            list.write(|i| i.push(val.clone()));
            Ok(Value::Empty)
        } else {
            Err(RuntimeError::TypeError(ErrorData::new(
                span,
                "Использование: list.append(value)".into(),
            )))
        }
    });

    // set(index: Number, value: Any) -> Empty
    define_method!(class_def, interner, "задать" => (_, args, span) {
        if let (Some(Value::List(list)), Some(raw_idx), Some(new_val)) = (
            CallArgListExt::first_value(&args),
            CallArgListExt::get_value(&args, 1),
            CallArgListExt::get_value(&args, 2),
        ) {
            list.write(|vec| {
                let idx = raw_idx.resolve_index(vec.len(), span)?;
                vec[idx] = new_val.clone();
                Ok(Value::Empty)
            })
        } else {
            Err(RuntimeError::TypeError(ErrorData::new(
                span,
                "Использование: list.set(number, value)".into(),
            )))
        }
    });

    // len() - Получить длину
    define_method!(class_def, interner, "длина" => (_, args, span) {
        if let Some(Value::List(list)) = CallArgListExt::first_value(&args) {
            let length = list.read(|i| i.len());
            Ok(Value::Number(length as i64))
        } else {
            Err(RuntimeError::TypeError(ErrorData::new(
                span,
                "Ожидался список".into(),
            )))
        }
    });

    // pop(index?) - Удалить и вернуть элемент (последний или по индексу)
    define_method!(class_def, interner, "удалить" => (_, args, span) {
        if let Some(Value::List(list)) = CallArgListExt::first_value(&args) {
            list.write(|vec| {
                if vec.is_empty() {
                    return Err(RuntimeError::InvalidOperation(ErrorData::new(
                        span,
                        "удаление у пустого списка".into(),
                    )));
                }

                let val = if let Some(raw_idx) = CallArgListExt::get_value(&args, 1) {
                    let idx = raw_idx.resolve_index(vec.len(), span)?;
                    vec.remove(idx)
                } else {
                    vec.pop().unwrap()
                };

                Ok(val)
            })
        } else {
            Err(RuntimeError::TypeError(ErrorData::new(
                span,
                "Ожидался список".into(),
            )))
        }
    });

    // clear() - Очистить список
    define_method!(class_def, interner, "отчистить" => (_, args, span) {
        if let Some(Value::List(list)) = CallArgListExt::first_value(&args) {
            list.write(|i| i.clear());
            Ok(Value::Empty)
        } else {
            Err(RuntimeError::TypeError(ErrorData::new(
                span,
                "Ожидался список".into(),
            )))
        }
    });

    // join(separator) - Склеить в строку
    define_method!(class_def, interner, "объединить" => (_, args, span) {
        if let (Some(Value::List(list)), Some(Value::Text(sep))) = (
            CallArgListExt::first_value(&args),
            CallArgListExt::get_value(&args, 1),
        ) {
            let joined = list.read(|i| {
                i.iter()
                    .map(|v| v.to_string())
                    .collect::<Vec<_>>()
                    .join(sep)
            });
            Ok(Value::Text(joined))
        } else {
            Err(RuntimeError::TypeError(ErrorData::new(
                span,
                "Использование: list.join(string)".into(),
            )))
        }
    });

    // get(index) - Безопасное получение (аналог list[i])
    define_method!(class_def, interner, "получить" => (_, args, span) {
        if let (Some(Value::List(list)), Some(idx)) = (
            CallArgListExt::first_value(&args),
            CallArgListExt::get_value(&args, 1),
        ) {
            list.read(|vec| {
                let i = idx.resolve_index(vec.len(), span)?;
                Ok(vec[i].clone())
            })
        } else {
            Err(RuntimeError::TypeError(ErrorData::new(
                span,
                "Использование: list.get(number)".into(),
            )))
        }
    });

    (name, SharedMut::new(class_def))
}

pub fn setup_list_func(interpreter: &mut Interpreter, interner: &SharedInterner) {
    define_builtin!(interpreter, interner, "список" => (_, arguments, _) {
        Ok(Value::List(SharedMut::new(
            arguments.into_iter().map(|arg| arg.value).collect(),
        )))
    });
}
