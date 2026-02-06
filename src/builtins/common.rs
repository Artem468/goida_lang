use std::sync::Arc;
use crate::ast::prelude::ErrorData;
use crate::interpreter::prelude::{BuiltinFn, Interpreter, RuntimeError, SharedInterner, Value};
use crate::traits::core::CoreOperations;

pub fn setup_type_func(interpreter: &mut Interpreter, interner: &SharedInterner) {
    interpreter.builtins.insert(
        interner.write(|i| i.get_or_intern("тип")),
        BuiltinFn(Arc::new(move |interpreter, arguments, span| {
            if arguments.len() != 1 {
                return Err(RuntimeError::InvalidOperation(ErrorData::new(
                    span,
                    format!(
                        "Функция 'тип' ожидает 1 аргумент, получено {}",
                        arguments.len()
                    ),
                )));
            }
            let val = arguments.get(0).ok_or_else(|| {
                RuntimeError::InvalidOperation(ErrorData::new(span, "Не передан объект".into()))
            })?;
            match val {
                Value::Number(_) => Ok(Value::Text("число".to_string())),
                Value::Float(_) => Ok(Value::Text("дробь".to_string())),
                Value::Text(_) => Ok(Value::Text("строка".to_string())),
                Value::Boolean(_) => Ok(Value::Text("логический".to_string())),
                Value::Object(obj) => Ok(Value::Text(format!(
                    "объект \"{}\"",
                    interpreter
                        .resolve_symbol(obj.read(|i| i.class_name))
                        .ok_or_else(|| RuntimeError::InvalidOperation(ErrorData::new(
                            span,
                            "Тип не найден".into()
                        )))?
                        .to_string()
                ))),
                Value::Class(cls) => Ok(Value::Text(format!(
                    "класс \"{}\"",
                    interpreter
                        .resolve_symbol(cls.read(|i| i.name))
                        .ok_or_else(|| RuntimeError::InvalidOperation(ErrorData::new(
                            span,
                            "Тип не найден".into()
                        )))?
                        .to_string()
                ))),
                Value::Function(obj) => Ok(Value::Text(format!(
                    "функция \"{}\"",
                    interpreter
                        .resolve_symbol(obj.name)
                        .ok_or_else(|| RuntimeError::InvalidOperation(ErrorData::new(
                            span,
                            "Тип не найден".into()
                        )))?
                        .to_string()
                ))),
                Value::Builtin(_) => Ok(Value::Text("встроенная функция".to_string())),
                Value::Module(sym) => Ok(Value::Text(format!(
                    "модуль \"{}\"",
                    interpreter.resolve_symbol(*sym).ok_or_else(|| {
                        RuntimeError::InvalidOperation(ErrorData::new(
                            span,
                            "Модуль не найден".into(),
                        ))
                    })?
                ))),
                Value::List(_) => Ok(Value::Text("список".to_string())),
                Value::Array(_) => Ok(Value::Text("массив".to_string())),
                Value::Dict(_) => Ok(Value::Text("словарь".to_string())),
                Value::NativeResource(_) => Ok(Value::Text("ресурс".to_string())),
                Value::Empty => Ok(Value::Text("пустота".to_string())),
            }
        })),
    );
}