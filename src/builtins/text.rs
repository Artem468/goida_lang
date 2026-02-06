use crate::ast::prelude::{ClassDefinition, ErrorData, Span, Visibility};
use crate::interpreter::prelude::{BuiltinFn, Interpreter, RuntimeError, SharedInterner, Value};
use crate::shared::SharedMut;
use std::sync::Arc;
use string_interner::DefaultSymbol as Symbol;

pub fn setup_text_class(interner: &SharedInterner) -> (Symbol, SharedMut<ClassDefinition>) {
    let name = interner.write(|i| i.get_or_intern("Строка"));

    let mut class_def = ClassDefinition::new(name, Span::default());

    class_def.set_constructor(BuiltinFn(Arc::new(|_interp, args, _span| {
        if let Some(Value::Object(instance)) = args.get(0) {
            let content = match args.get(1) {
                Some(Value::Text(s)) => s.clone(),
                Some(Value::Number(n)) => n.to_string(),
                Some(Value::Float(f)) => f.to_string(),
                Some(Value::Boolean(b)) => b.to_string(),
                _ => String::new(),
            };

            let data_sym = _interp.interner.write(|i| i.get_or_intern("__data"));
            instance.write(|i| i.field_values.insert(data_sym, Value::Text(content)));
        }
        Ok(Value::Empty)
    })));

    // len() -> Number
    class_def.add_method(
        interner.write(|i| i.get_or_intern("длина")),
        Visibility::Public,
        false,
        BuiltinFn(Arc::new(|_interp, args, span| {
            if let Some(Value::Text(s)) = args.get(0) {
                Ok(Value::Number(s.chars().count() as i64))
            } else {
                Err(RuntimeError::TypeError(ErrorData::new(
                    span,
                    "Ожидалась строка".into(),
                )))
            }
        })),
    );

    // split(separator: Text) -> List
    class_def.add_method(
        interner.write(|i| i.get_or_intern("разделить")),
        Visibility::Public,
        false,
        BuiltinFn(Arc::new(|_interp, args, span| {
            if let (Some(Value::Text(s)), Some(Value::Text(sep))) = (args.get(0), args.get(1)) {
                let parts: Vec<Value> = s
                    .split(sep)
                    .map(|part| Value::Text(part.to_string()))
                    .collect();
                Ok(Value::List(SharedMut::new(parts)))
            } else {
                Err(RuntimeError::TypeError(ErrorData::new(
                    span,
                    "Использование: str.split(separator)".into(),
                )))
            }
        })),
    );

    // upper() -> Text
    class_def.add_method(
        interner.write(|i| i.get_or_intern("верхний")),
        Visibility::Public,
        false,
        BuiltinFn(Arc::new(|_interp, args, span| {
            if let Some(Value::Text(s)) = args.get(0) {
                Ok(Value::Text(s.to_uppercase()))
            } else {
                Err(RuntimeError::TypeError(ErrorData::new(
                    span,
                    "Ожидалась строка".into(),
                )))
            }
        })),
    );

    // lower() -> Text
    class_def.add_method(
        interner.write(|i| i.get_or_intern("нижний")),
        Visibility::Public,
        false,
        BuiltinFn(Arc::new(|_interp, args, span| {
            if let Some(Value::Text(s)) = args.get(0) {
                Ok(Value::Text(s.to_lowercase()))
            } else {
                Err(RuntimeError::TypeError(ErrorData::new(
                    span,
                    "Ожидалась строка".into(),
                )))
            }
        })),
    );

    // contains(substring: Text) -> Boolean
    class_def.add_method(
        interner.write(|i| i.get_or_intern("содержит")),
        Visibility::Public,
        false,
        BuiltinFn(Arc::new(|_interp, args, span| {
            if let (Some(Value::Text(s)), Some(Value::Text(sub))) = (args.get(0), args.get(1)) {
                Ok(Value::Boolean(s.contains(sub)))
            } else {
                Err(RuntimeError::TypeError(ErrorData::new(
                    span,
                    "Использование: str.contains(substring)".into(),
                )))
            }
        })),
    );

    // replace(old: Text, new: Text) -> Text
    class_def.add_method(
        interner.write(|i| i.get_or_intern("заменить")),
        Visibility::Public,
        false,
        BuiltinFn(Arc::new(|_interp, args, span| {
            if let (Some(Value::Text(s)), Some(Value::Text(old)), Some(Value::Text(new))) =
                (args.get(0), args.get(1), args.get(2))
            {
                Ok(Value::Text(s.replace(old, new)))
            } else {
                Err(RuntimeError::TypeError(ErrorData::new(
                    span,
                    "Использование: str.replace(old, new)".into(),
                )))
            }
        })),
    );

    (name, SharedMut::new(class_def))
}


pub fn setup_text_func(interpreter: &mut Interpreter, interner: &SharedInterner) {
    interpreter.builtins.insert(
        interner.write(|i| i.get_or_intern("строка")),
        BuiltinFn(Arc::new(move |_interpreter, arguments, span| {
            if arguments.len() != 1 {
                return Err(RuntimeError::InvalidOperation(ErrorData::new(
                    span,
                    format!(
                        "Функция 'строка' ожидает 1 аргумент, получено {}",
                        arguments.len()
                    ),
                )));
            }
            let n: String = arguments[0].clone().try_into()?;
            Ok(Value::Text(n))
        })),
    );
}