use crate::interpreter::prelude::RuntimeError;
use crate::interpreter::structs::Value;
use std::fmt;
use std::rc::Rc;

impl Value {
    pub fn to_string(&self) -> String {
        match self {
            Value::Number(n) => n.to_string(),
            Value::Float(n) => n.to_string(),
            Value::Text(s) => s.clone(),
            Value::Boolean(b) => {
                if *b {
                    "истина".to_string()
                } else {
                    "ложь".to_string()
                }
            }
            Value::Object(obj) => format!("Объект {:p}", Rc::as_ptr(obj)),
            Value::Function(func) => format!("Функция {:p}", Rc::as_ptr(func)),
            Value::Builtin(func) => format!("Встроенная функция {:p}", func),
            Value::Empty => "пустота".to_string(),
        }
    }

    pub(crate) fn is_truthy(&self) -> bool {
        match self {
            Value::Boolean(b) => *b,
            Value::Number(n) => *n != 0,
            Value::Float(n) => *n != 0.0,
            Value::Text(s) => !s.is_empty(),
            Value::Object(_) => true,
            Value::Function(_) => true,
            Value::Builtin(_) => true,
            Value::Empty => false,
        }
    }
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Value::Number(data) => {
                write!(f, "{data}")
            }
            Value::Float(data) => {
                write!(f, "{data}")
            }
            Value::Text(data) => {
                write!(f, "{data}")
            }
            Value::Boolean(data) => {
                write!(f, "{data}")
            }
            Value::Object(obj) => {
                write!(f, "[Объект {:p}]", Rc::as_ptr(obj))
            }
            Value::Function(func) => {
                write!(f, "[Функция {:p}]", Rc::as_ptr(func))
            }
            Value::Builtin(func) => {
                write!(f, "[Встроенная функция {:p}]", func)
            }
            Value::Empty => {
                write!(f, "Пустота")
            }
        }
    }
}

impl TryFrom<Value> for f64 {
    type Error = RuntimeError;

    fn try_from(value: Value) -> Result<Self, Self::Error> {
        match value {
            Value::Float(data) => Ok(data),
            Value::Number(data) => Ok(data as f64),
            Value::Text(data) => data
                .parse()
                .map_err(|_| RuntimeError::TypeError(format!("Не удалось преобразовать текст '{}' в дробное число", data))),
            Value::Boolean(b) => Ok(if b { 1.0 } else { 0.0 }),
            _ => Err(RuntimeError::TypeError("Тип не может быть приведен к дробному числу".into())),
        }
    }
}

impl TryFrom<Value> for i64 {
    type Error = RuntimeError;

    fn try_from(value: Value) -> Result<Self, Self::Error> {
        match value {
            Value::Float(data) => Ok(data as i64),
            Value::Number(data) => Ok(data),
            Value::Text(data) => data
                .parse()
                .map_err(|_| RuntimeError::TypeError(format!("Не удалось преобразовать текст '{}' в целое число", data))),
            Value::Boolean(b) => Ok(if b { 1 } else { 0 }),
            _ => Err(RuntimeError::TypeError("Тип не может быть приведен к целому числу".into())),
        }
    }
}

impl TryFrom<Value> for String {
    type Error = RuntimeError;

    fn try_from(value: Value) -> Result<Self, Self::Error> {
        Ok(value.to_string())
    }
}

impl TryFrom<Value> for bool {
    type Error = RuntimeError;

    fn try_from(value: Value) -> Result<Self, Self::Error> {
        match value {
            Value::Boolean(b) => Ok(b),
            Value::Empty => Ok(false),
            Value::Number(n) => Ok(n != 0),
            Value::Float(f) => Ok(f != 0.0 && !f.is_nan()),
            Value::Text(s) => Ok(!s.is_empty()),
            Value::Object(_) | Value::Function(_) | Value::Builtin(_) => Ok(true),
        }
    }
}