use crate::interpreter::prelude::Value;
use crate::shared::SharedMut;
use serde_json::Value as JsonValue;
use std::collections::HashMap;

pub trait JsonParsable {
    fn from_json(json: JsonValue) -> Value;
    fn to_json(&self) -> Result<JsonValue, String>;
}

impl JsonParsable for Value {
    fn from_json(json: JsonValue) -> Value {
        match json {
            JsonValue::Null => Value::Empty,
            JsonValue::Bool(b) => Value::Boolean(b),
            JsonValue::Number(n) => {
                if let Some(i) = n.as_i64() {
                    Value::Number(i)
                } else {
                    Value::Float(n.as_f64().unwrap_or(0.0))
                }
            }
            JsonValue::String(s) => Value::Text(s),
            JsonValue::Array(arr) => {
                let list = arr.into_iter().map(Value::from_json).collect();
                Value::List(SharedMut::new(list))
            }
            JsonValue::Object(obj) => {
                let mut dict = HashMap::new();
                for (k, v) in obj {
                    dict.insert(k, Value::from_json(v));
                }
                Value::Dict(SharedMut::new(dict))
            }
        }
    }

    fn to_json(&self) -> Result<JsonValue, String> {
        match self {
            Value::Empty => Ok(JsonValue::Null),
            Value::Boolean(value) => Ok(JsonValue::Bool(*value)),
            Value::Number(value) => Ok(JsonValue::Number((*value).into())),
            Value::Float(value) => serde_json::Number::from_f64(*value)
                .map(JsonValue::Number)
                .ok_or_else(|| format!("Нельзя сериализовать число '{}' в JSON", value)),
            Value::Text(value) => Ok(JsonValue::String(value.clone())),
            Value::List(items) => items.read(|items| {
                items
                    .iter()
                    .map(Value::to_json)
                    .collect::<Result<Vec<_>, _>>()
                    .map(JsonValue::Array)
            }),
            Value::Array(items) => items
                .iter()
                .map(Value::to_json)
                .collect::<Result<Vec<_>, _>>()
                .map(JsonValue::Array),
            Value::Dict(items) => items.read(|items| {
                let mut map = serde_json::Map::with_capacity(items.len());
                for (key, value) in items {
                    map.insert(key.clone(), value.to_json()?);
                }
                Ok(JsonValue::Object(map))
            }),
            Value::Object(_) => Err("Нельзя сериализовать объект класса в JSON".into()),
            Value::Class(_) => Err("Нельзя сериализовать класс в JSON".into()),
            Value::Function(_) => Err("Нельзя сериализовать функцию в JSON".into()),
            Value::Builtin(_) => Err("Нельзя сериализовать встроенную функцию в JSON".into()),
            Value::Module(_) => Err("Нельзя сериализовать модуль в JSON".into()),
            Value::NativeResource(_) => Err("Нельзя сериализовать нативный ресурс в JSON".into()),
        }
    }
}
