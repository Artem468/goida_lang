use crate::ast::prelude::{ClassDefinition, ErrorData, Span};
use crate::builtins::iterator::values_from_iterable;
use crate::interpreter::prelude::{
    CallArgListExt, Interpreter, RuntimeError, RuntimeIterator, SharedInterner, Value,
};
use crate::shared::SharedMut;
use crate::traits::prelude::CoreOperations;
use crate::{bail_runtime, define_builtin, define_constructor, define_method, runtime_error};
use string_interner::DefaultSymbol as Symbol;

pub fn setup_list_class(interner: &SharedInterner) -> (Symbol, SharedMut<ClassDefinition>) {
    let name =
        interner.write(|i| i.get_or_intern(crate::builtins::catalog::class::LIST.names.canonical));

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
    define_method!(class_def, interner, crate::builtins::catalog::method::ADD.canonical => (_, args, span) {
        if let (Some(Value::List(list)), Some(val)) = (
            CallArgListExt::first_value(&args),
            CallArgListExt::get_value(&args, 1),
        ) {
            list.write(|i| i.push(val.clone()));
            Ok(Value::Empty)
        } else {
            bail_runtime!(
                TypeError,
                span,
                "Использование: list.append(value)"
            )
        }
    });

    // set(index: Number, value: Any) -> Empty
    define_method!(class_def, interner, crate::builtins::catalog::method::SET.canonical => (_, args, span) {
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
            bail_runtime!(TypeError, span, "Использование: list.set(number, value)")
        }
    });

    // len() - Получить длину
    define_method!(class_def, interner, crate::builtins::catalog::method::LEN.canonical => (_, args, span) {
        if let Some(Value::List(list)) = CallArgListExt::first_value(&args) {
            let length = list.read(|i| i.len());
            Ok(Value::Number(length as i64))
        } else {
            bail_runtime!(TypeError, span, "Ожидался список")
        }
    });

    // pop(index?) - Удалить и вернуть элемент (последний или по индексу)
    define_method!(class_def, interner, crate::builtins::catalog::method::REMOVE.canonical => (_, args, span) {
        if let Some(Value::List(list)) = CallArgListExt::first_value(&args) {
            list.write(|vec| {
                if vec.is_empty() {
                    return bail_runtime!(InvalidOperation, span, "удаление у пустого списка");
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
            bail_runtime!(TypeError, span, "Ожидался список")
        }
    });

    // clear() - Очистить список
    define_method!(class_def, interner, crate::builtins::catalog::method::CLEAR_TYPO.canonical => (_, args, span) {
        if let Some(Value::List(list)) = CallArgListExt::first_value(&args) {
            list.write(|i| i.clear());
            Ok(Value::Empty)
        } else {
            bail_runtime!(TypeError, span, "Ожидался список")
        }
    });

    // join(separator) - Склеить в строку
    define_method!(class_def, interner, crate::builtins::catalog::method::JOIN.canonical => (_, args, span) {
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
            bail_runtime!(TypeError, span, "Использование: list.join(string)")
        }
    });

    // get(index) - Безопасное получение (аналог list[i])
    define_method!(class_def, interner, crate::builtins::catalog::method::GET.canonical => (_, args, span) {
        if let (Some(Value::List(list)), Some(idx)) = (
            CallArgListExt::first_value(&args),
            CallArgListExt::get_value(&args, 1),
        ) {
            list.read(|vec| {
                let i = idx.resolve_index(vec.len(), span)?;
                Ok(vec[i].clone())
            })
        } else {
            bail_runtime!(TypeError, span, "Использование: list.get(number)")
        }
    });

    define_method!(class_def, interner, crate::builtins::catalog::method::ITERATOR.canonical => (_, args, span) {
        let Some(value) = CallArgListExt::first_value(&args) else {
            return bail_runtime!(TypeError, span, "Ожидался список");
        };
        Ok(Value::Iterator(RuntimeIterator::new(values_from_iterable(value, span)?)))
    });

    (name, SharedMut::new(class_def))
}

pub fn setup_list_func(interpreter: &mut Interpreter, interner: &SharedInterner) {
    define_builtin!(interpreter, interner, crate::builtins::catalog::function::LIST.canonical => (_, arguments, _) {
        Ok(Value::List(SharedMut::new(
            arguments.into_iter().map(|arg| arg.value).collect(),
        )))
    });
}
