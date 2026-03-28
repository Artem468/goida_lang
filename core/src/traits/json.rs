use serde_json::Value as JsonValue;
use crate::interpreter::prelude::Value;
use crate::shared::SharedMut;
use std::collections::HashMap;

pub trait JsonParsable {
    fn from_json(json: JsonValue) -> Value;
    fn to_json(&self) -> Result<JsonValue, String>; // На будущее, если захочешь Сеть.отправить(Dict)
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
        // Тут логика обратной конвертации Value -> JsonValue
        // пригодится для метода Запрос.тело(словарь)
        todo!()
    }
}
