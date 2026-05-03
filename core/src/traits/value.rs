use crate::ast::prelude::{ErrorData, Span};
use crate::ast::program::FieldData;
use crate::interpreter::prelude::{ClassInstance, RuntimeError};
use crate::interpreter::structs::Value;
use crate::shared::SharedMut;
use crate::{bail_runtime, runtime_error, INTERNER};
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
            Value::NativeGlobal(_) => true,
            Value::Empty => false,
        }
    }
    pub fn as_index(&self) -> Result<i64, String> {
        match self {
            Value::Number(n) => Ok(*n),
            _ => Err(format!("Ожидалось число (индекс), но получено {:?}", self)),
        }
    }

    pub fn as_i64(&self) -> Option<i64> {
        match self {
            Value::Number(n) => Some(*n),
            Value::Float(f) => Some(*f as i64),
            _ => None,
        }
    }

    pub fn as_str(&self) -> Option<&String> {
        if let Value::Text(s) = self {
            Some(s)
        } else {
            None
        }
    }

    pub fn as_object(&self, span: Span) -> Result<SharedMut<ClassInstance>, RuntimeError> {
        if let Value::Object(obj) = self {
            Ok(obj.clone())
        } else {
            bail_runtime!(TypeError, span, "Ожидался объект")
        }
    }

    pub fn resolve_index(&self, len: usize, span: Span) -> Result<usize, RuntimeError> {
        let raw_idx = match self {
            Value::Number(n) => *n,
            _ => {
                return bail_runtime!(
                    TypeError,
                    span,
                    "Индекс должен быть числом, получено {:?}",
                    self
                )
            }
        };

        let final_idx = if raw_idx < 0 {
            let abs_idx = raw_idx.unsigned_abs() as usize;
            if abs_idx > len {
                return bail_runtime!(
                    InvalidOperation,
                    span,
                    "Отрицательный индекс {} слишком велик (длина {})",
                    raw_idx,
                    len
                );
            }
            len - abs_idx
        } else {
            raw_idx as usize
        };

        if final_idx >= len {
            return bail_runtime!(
                InvalidOperation,
                span,
                "Индекс {} вне границ (длина {})",
                raw_idx,
                len
            );
        }

        Ok(final_idx)
    }
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Value::Number(n) => write!(f, "{}", n),
            Value::Float(n) => write!(f, "{}", n),
            Value::Text(s) => write!(f, "{}", s),
            Value::Boolean(b) => write!(f, "{}", if *b { "истина" } else { "ложь" }),
            Value::Object(obj) => INTERNER.read(|i| {
                let cls_name = i
                    .resolve(obj.read(|o| o.class_name))
                    .unwrap_or("неизвестно");
                write!(f, "<Объект \"{}\" {:p}>", cls_name, obj)
            }),
            Value::Class(cls) => INTERNER.read(|i| {
                let cls_name = i.resolve(cls.read(|c| c.name)).unwrap_or("неизвестно");
                write!(f, "<Класс \"{}\" {:p}>", cls_name, cls)
            }),
            Value::Function(func) => INTERNER.read(|i| {
                let fun_name = i.resolve(func.as_ref().name).unwrap_or("неизвестно");
                write!(f, "<Функция {} {:p}>", fun_name, func)
            }),
            Value::Builtin(func) => write!(f, "<Встроенная функция {:p}>", func),
            Value::Module(module) => INTERNER.read(|i| {
                let module_name = i.resolve(*module).unwrap_or("неизвестно");
                write!(f, "<Модуль {}>", module_name)
            }),
            Value::List(list) => list.read(|items| {
                write!(f, "[")?;
                for (i, item) in items.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", item)?;
                }
                write!(f, "]")
            }),
            Value::Array(array) => {
                write!(f, "[")?;
                for (i, item) in array.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", item)?;
                }
                write!(f, "]")
            }
            Value::Dict(dict) => dict.read(|items| {
                let mut pairs: Vec<_> = items.iter().collect();
                pairs.sort_by_key(|(k, _)| *k);
                write!(f, "{{")?;
                for (i, (k, v)) in pairs.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "\"{}\": {}", k, v)?;
                }
                write!(f, "}}")
            }),
            Value::NativeResource(resource) => write!(f, "<Ресурс {:p}>", resource),
            Value::NativeGlobal(binding) => INTERNER.read(|i| {
                let binding_name = i.resolve(binding.symbol_name).unwrap_or("неизвестно");
                write!(f, "<Нативная переменная {}>", binding_name)
            }),
            Value::Empty => write!(f, "пустота"),
        }
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
            | Value::NativeResource(_)
            | Value::NativeGlobal(_) => Ok(true),
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
            (Value::NativeGlobal(a), Value::NativeGlobal(b)) => Arc::ptr_eq(a, b),
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
