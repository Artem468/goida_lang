use crate::ast::prelude::{ClassDefinition, ErrorData, Span, Visibility};
use crate::interpreter::prelude::{
    BuiltinFn, CallArgListExt, Interpreter, RuntimeError, SharedInterner, Value,
};
use crate::shared::SharedMut;
use std::sync::Arc;
use string_interner::DefaultSymbol as Symbol;

pub fn setup_array_class(interner: &SharedInterner) -> (Symbol, SharedMut<ClassDefinition>) {
    let name = interner.write(|i| i.get_or_intern("Массив"));

    let mut class_def = ClassDefinition::new(name, Span::default());

    class_def.set_constructor(BuiltinFn(Arc::new(|_interp, args, _span| {
        if let Some(Value::Object(instance)) = CallArgListExt::first_value(&args) {
            let items = args[1..].iter().map(|arg| arg.value.clone()).collect();
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
            if let Some(Value::Array(arr)) = CallArgListExt::first_value(&args) {
                let length = arr.as_ref().len();
                Ok(Value::Number(length as i64))
            } else {
                Err(RuntimeError::TypeError(ErrorData::new(
                    span,
                    "Ожидался Array".into(),
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
            if let (Some(Value::Array(arr)), Some(Value::Text(sep))) = (
                CallArgListExt::first_value(&args),
                CallArgListExt::get_value(&args, 1),
            ) {
                let res = arr
                    .as_ref()
                    .iter()
                    .map(|v| v.to_string())
                    .collect::<Vec<_>>()
                    .join(sep);

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
            if let (Some(Value::Array(arr)), Some(idx)) = (
                CallArgListExt::first_value(&args),
                CallArgListExt::get_value(&args, 1),
            ) {
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

pub fn setup_array_func(interpreter: &mut Interpreter, interner: &SharedInterner) {
    interpreter.builtins.insert(
        interner.write(|i| i.get_or_intern("массив")),
        BuiltinFn(Arc::new(move |_interpreter, arguments, _span| {
            Ok(Value::Array(Arc::new(
                arguments.into_iter().map(|arg| arg.value).collect(),
            )))
        })),
    );
}
