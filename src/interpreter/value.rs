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
            Value::Empty => {
                write!(f, "")
            }
        }
    }
}