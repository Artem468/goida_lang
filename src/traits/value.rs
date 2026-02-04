use crate::ast::prelude::{ErrorData, Span};
use crate::ast::program::FieldData;
use crate::interpreter::prelude::RuntimeError;
use crate::interpreter::structs::Value;
use crate::shared::SharedMut;
use std::fmt;
use std::sync::Arc;

pub trait ValueOperations {
    fn add_values(&self, left: Value, right: Value, span: Span) -> Result<Value, RuntimeError>;
    fn subtract_values(&self, left: Value, right: Value, span: Span)
        -> Result<Value, RuntimeError>;
    fn multiply_values(&self, left: Value, right: Value, span: Span)
        -> Result<Value, RuntimeError>;
    fn divide_values(&self, left: Value, right: Value, span: Span) -> Result<Value, RuntimeError>;
    fn modulo_values(&self, left: Value, right: Value, span: Span) -> Result<Value, RuntimeError>;
    fn compare_greater(&self, left: Value, right: Value, span: Span)
        -> Result<Value, RuntimeError>;
    fn compare_less(&self, left: Value, right: Value, span: Span) -> Result<Value, RuntimeError>;
    fn compare_greater_equal(
        &self,
        left: Value,
        right: Value,
        span: Span,
    ) -> Result<Value, RuntimeError>;
    fn compare_less_equal(
        &self,
        left: Value,
        right: Value,
        span: Span,
    ) -> Result<Value, RuntimeError>;
}

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
            Value::Object(obj) => format!("<Объект {:p}>", obj),
            Value::Class(cls) => format!("<Класс {:p}>", cls),
            Value::Function(func) => format!("<Функция {:p}>", func),
            Value::Builtin(func) => format!("<Встроенная функция {:p}>", func),
            Value::Module(_) => "<Модуль>".to_string(),
            Value::List(list) => {
                list.read(|items| {
                    let strings: Vec<String> = items.iter().map(|v| v.to_string()).collect();
                    format!("[{}]", strings.join(", "))
                })
            }
            Value::Array(array) => {
                let strings: Vec<String> = array.iter().map(|v| v.to_string()).collect();
                format!("[{}]", strings.join(", "))
            }
            Value::Dict(dict) => {
                dict.read(|items| {
                    let mut pairs: Vec<String> = items
                        .iter()
                        .map(|(k, v)| format!("\"{}\": {}", k, v.to_string()))
                        .collect();
                    pairs.sort();
                    format!("{{{}}}", pairs.join(", "))
                })
            }
            Value::NativeResource(resource) => format!("<Ресурс {:p}>", resource),
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
            Value::Class(_) => true,
            Value::Function(_) => true,
            Value::Builtin(_) => true,
            Value::Module(_) => true,
            Value::List(list) => !list.read(|l| l.is_empty()),
            Value::Array(array) => !array.is_empty(),
            Value::Dict(dict) => !dict.read(|d| d.is_empty()),
            Value::NativeResource(_) => true,
            Value::Empty => false,
        }
    }
    pub fn as_index(&self) -> Result<i64, String> {
        match self {
            Value::Number(n) => Ok(*n),
            _ => Err(format!("Ожидалось число (индекс), но получено {:?}", self)),
        }
    }

    pub fn resolve_index(&self, len: usize, span: Span) -> Result<usize, RuntimeError> {
        let raw_idx = match self {
            Value::Number(n) => *n,
            _ => return Err(RuntimeError::TypeError(ErrorData::new(
                span,
                format!("Индекс должен быть числом, получено {:?}", self)
            ))),
        };

        let final_idx = if raw_idx < 0 {
            let abs_idx = raw_idx.unsigned_abs() as usize;
            if abs_idx > len {
                return Err(RuntimeError::InvalidOperation(ErrorData::new(
                    span,
                    format!("Отрицательный индекс {} слишком велик (длина {})", raw_idx, len)
                )));
            }
            len - abs_idx
        } else {
            raw_idx as usize
        };

        if final_idx >= len {
            return Err(RuntimeError::InvalidOperation(ErrorData::new(
                span,
                format!("Индекс {} вне границ (длина {})", raw_idx, len)
            )));
        }

        Ok(final_idx)
    }
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.to_string().fmt(f)
    }
}

impl TryFrom<Value> for f64 {
    type Error = String;

    fn try_from(value: Value) -> Result<Self, Self::Error> {
        match value {
            Value::Float(data) => Ok(data),
            Value::Number(data) => Ok(data as f64),
            Value::Text(data) => data
                .parse()
                .map_err(|_| format!("Не удалось преобразовать строку '{}' в дробное число", data)),
            Value::Boolean(b) => Ok(if b { 1.0 } else { 0.0 }),
            _ => Err("Тип не может быть приведен к дробному числу".into()),
        }
    }
}

impl TryFrom<Value> for i64 {
    type Error = String;

    fn try_from(value: Value) -> Result<Self, Self::Error> {
        match value {
            Value::Float(data) => Ok(data as i64),
            Value::Number(data) => Ok(data),
            Value::Text(data) => data
                .parse()
                .map_err(|_| format!("Не удалось преобразовать строку '{}' в целое число", data)),
            Value::Boolean(b) => Ok(if b { 1 } else { 0 }),
            _ => Err("Тип не может быть приведен к целому числу".into()),
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
            Value::List(list) => Ok(!list.read(|l| l.is_empty())),
            Value::Array(array) => Ok(!array.is_empty()),
            Value::Dict(dict) => Ok(!dict.read(|d| d.is_empty())),
            Value::Object(_)
            | Value::Class(_)
            | Value::Function(_)
            | Value::Builtin(_)
            | Value::Module(_)
            | Value::NativeResource(_) => Ok(true),
        }
    }
}

impl PartialEq for Value {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Value::Number(a), Value::Number(b)) => a == b,
            (Value::Float(a), Value::Float(b)) => a == b,
            (Value::Text(a), Value::Text(b)) => a == b,
            (Value::Boolean(a), Value::Boolean(b)) => a == b,
            (Value::Object(a), Value::Object(b)) => a.ptr_eq(b),
            (Value::Function(a), Value::Function(b)) => Arc::ptr_eq(a, b),
            (Value::Module(a), Value::Module(b)) => a == b,
            (Value::List(a), Value::List(b)) => a.ptr_eq(b),
            (Value::Array(a), Value::Array(b)) => Arc::ptr_eq(a, b),
            (Value::Dict(a), Value::Dict(b)) => a.ptr_eq(b),
            (Value::Empty, Value::Empty) => true,
            _ => false,
        }
    }
}

impl From<SharedMut<Value>> for FieldData {
    fn from(lock: SharedMut<Value>) -> Self {
        FieldData::Value(lock)
    }
}
