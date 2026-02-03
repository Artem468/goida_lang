use crate::ast::prelude::Span;
use crate::interpreter::prelude::RuntimeError;
use crate::interpreter::structs::Value;
use std::fmt;
use std::rc::Rc;

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
            Value::Function(func) => format!("<Функция {:p}>", func),
            Value::Builtin(func) => format!("<Встроенная функция {:p}>", func),
            Value::Module(_) => "<Модуль>".to_string(),
            Value::List(list) => {
                let items = list.borrow();
                let strings: Vec<String> = items.iter().map(|v| v.to_string()).collect();
                format!("[{}]", strings.join(", "))
            }
            Value::Array(array) => {
                let strings: Vec<String> = array.iter().map(|v| v.to_string()).collect();
                format!("[{}]", strings.join(", "))
            }
            Value::Dict(dict) => {
                let items = dict.borrow();
                let mut pairs: Vec<String> = items
                    .iter()
                    .map(|(k, v)| format!("\"{}\": {}", k, v.to_string()))
                    .collect();
                pairs.sort();
                format!("{{{}}}", pairs.join(", "))
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
            Value::Function(_) => true,
            Value::Builtin(_) => true,
            Value::Module(_) => true,
            Value::List(list) => !list.borrow().is_empty(),
            Value::Array(array) => !array.is_empty(),
            Value::Dict(dict) => !dict.borrow().is_empty(),
            Value::NativeResource(_) => true,
            Value::Empty => false,
        }
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
            Value::Text(data) => data.parse().map_err(|_| {
                format!("Не удалось преобразовать строку '{}' в целое число", data)
            }),
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
            Value::List(list) => Ok(!list.borrow().is_empty()),
            Value::Array(array) => Ok(!array.is_empty()),
            Value::Dict(dict) => Ok(!dict.borrow().is_empty()),
            Value::Object(_) | Value::Function(_) | Value::Builtin(_) | Value::Module(_) | Value::NativeResource(_) => {
                Ok(true)
            }
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
            (Value::Object(a), Value::Object(b)) => Rc::ptr_eq(a, b),
            (Value::Function(a), Value::Function(b)) => Rc::ptr_eq(a, b),
            (Value::Module(a), Value::Module(b)) => a == b,
            (Value::List(a), Value::List(b)) => Rc::ptr_eq(a, b),
            (Value::Array(a), Value::Array(b)) => Rc::ptr_eq(a, b),
            (Value::Dict(a), Value::Dict(b)) => Rc::ptr_eq(a, b),
            (Value::Empty, Value::Empty) => true,
            _ => false,
        }
    }
}
