use crate::ast::prelude::{ClassDefinition, ErrorData, Span};
use crate::interpreter::prelude::{
    CallArgListExt, Interpreter, RuntimeError, SharedInterner, Value,
};
use crate::shared::SharedMut;
use crate::{bail_runtime, define_builtin, define_constructor, define_method, runtime_error};
use std::sync::Arc;
use string_interner::DefaultSymbol as Symbol;

pub fn setup_array_class(interner: &SharedInterner) -> (Symbol, SharedMut<ClassDefinition>) {
    let name = interner.write(|i| i.get_or_intern("Массив"));

    let mut class_def = ClassDefinition::new(name, Span::default());

    define_constructor!(class_def, (interp, args, _) {
        if let Some(Value::Object(instance)) = CallArgListExt::first_value(&args) {
            let items: Vec<Value> = args[1..].iter().map(|arg| arg.value.clone()).collect();
            let internal_array = Value::Array(Arc::new(items));

            let data_sym = interp.interner.write(|i| i.get_or_intern("__data"));
            instance.write(|i| i.field_values.insert(data_sym, internal_array));
        }
        Ok(Value::Empty)
    });

    // len() - Получить длину
    define_method!(class_def, interner, "длина" => (_, args, span) {
        if let Some(Value::Array(arr)) = CallArgListExt::first_value(&args) {
            let length = arr.len();
            Ok(Value::Number(length as i64))
        } else {
            bail_runtime!(
                TypeError,
                span,
                "Ожидался массив"
            )
        }
    });

    // join(separator) - Склеить в строку
    define_method!(class_def, interner, "объединить" => (_, args, span) {
        if let (Some(Value::Array(arr)), Some(Value::Text(sep))) = (
            CallArgListExt::first_value(&args),
            CallArgListExt::get_value(&args, 1),
        ) {
            let res = arr
                .iter()
                .map(|v| v.to_string())
                .collect::<Vec<_>>()
                .join(sep);

            Ok(Value::Text(res))
        } else {
            bail_runtime!(
                TypeError,
                span,
                "Использование: array.join(string)"
            )
        }
    });

    // get(index) - Безопасное получение (аналог list[i])
    define_method!(class_def, interner, "получить" => (_, args, span) {
        if let (Some(Value::Array(arr)), Some(idx)) = (
            CallArgListExt::first_value(&args),
            CallArgListExt::get_value(&args, 1),
        ) {
            let i = idx.resolve_index(arr.len(), span)?;
            Ok(arr[i].clone())
        } else {
            bail_runtime!(
                TypeError,
                span,
                "Использование: array.get(number)"
            )
        }
    });

    (name, SharedMut::new(class_def))
}

pub fn setup_array_func(interpreter: &mut Interpreter, interner: &SharedInterner) {
    define_builtin!(interpreter, interner, "массив" => (_, arguments, _span) {
        Ok(Value::Array(Arc::new(
            arguments.into_iter().map(|arg| arg.value).collect(),
        )))
    });
}
