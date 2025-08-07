use std::fmt;
use crate::interpreter::structs::Value;

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
            Value::List(items) => {
                let mut result = String::from("[");
                for (i, item) in items.iter().enumerate() {
                    if i > 0 {
                        result.push_str(", ");
                    }
                    result.push_str(&item.to_string());
                }
                result.push(']');
                result
            }
            Value::Dict(map) => {
                let mut result = String::from("{");
                let mut first = true;
                for (key, value) in map {
                    if !first {
                        result.push_str(", ");
                    }
                    result.push_str(&format!("{}: {}", key, value.to_string()));
                    first = false;
                }
                result.push('}');
                result
            }
            Value::Empty => "пустота".to_string(),
        }
    }

    pub(crate) fn is_truthy(&self) -> bool {
        match self {
            Value::Boolean(b) => *b,
            Value::Number(n) => *n != 0,
            Value::Float(n) => *n != 0.0,
            Value::Text(s) => !s.is_empty(),
            Value::List(items) => !items.is_empty(),
            Value::Dict(map) => !map.is_empty(),
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
            Value::List(_) | Value::Dict(_) => {
                write!(f, "{}", self.to_string())
            }
            Value::Empty => {
                write!(f, "")
            }
        }
    }
}
