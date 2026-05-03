use crate::ast::prelude::{ErrorData, Span};
use crate::interpreter::structs::{Interpreter, RuntimeError, Value};
use crate::shared::SharedMut;
use crate::traits::prelude::ValueOperations;
use crate::{bail_runtime, runtime_error};
use std::sync::Arc;

impl ValueOperations for Interpreter {
    fn add_values(&self, left: Value, right: Value, span: Span) -> Result<Value, RuntimeError> {
        match (&left, &right) {
            (Value::Number(a), Value::Number(b)) => Ok(Value::Number(a + b)),
            (Value::Float(a), Value::Float(b)) => Ok(Value::Float(a + b)),
            (Value::Number(a), Value::Float(b)) => Ok(Value::Float(*a as f64 + *b)),
            (Value::Float(a), Value::Number(b)) => Ok(Value::Float(*a + *b as f64)),

            (Value::Text(a), Value::Text(b)) => Ok(Value::Text(format!("{}{}", a, b))),
            (Value::Text(a), any) => Ok(Value::Text(format!("{}{}", a, any))),
            (any, Value::Text(b)) => Ok(Value::Text(format!("{}{}", any, b))),

            (Value::List(a), Value::List(b)) => {
                let new_vec = a.read(|vec_a| {
                    b.read(|vec_b| {
                        let mut combined = vec_a.clone();
                        combined.extend_from_slice(vec_b);
                        combined
                    })
                });
                Ok(Value::List(SharedMut::new(new_vec)))
            }

            (Value::Dict(a), Value::Dict(b)) => {
                let new_dict = a.read(|dict_a| {
                    b.read(|dict_b| {
                        let mut combined = dict_a.clone();
                        for (k, v) in dict_b {
                            combined.insert(k.clone(), v.clone());
                        }
                        combined
                    })
                });

                Ok(Value::Dict(SharedMut::new(new_dict)))
            }

            (Value::Array(a), Value::Array(b)) => {
                let mut new_vec = (**a).clone();
                new_vec.extend_from_slice(b);
                Ok(Value::Array(Arc::new(new_vec)))
            }

            _ => bail_runtime!(
                TypeMismatch,
                span,
                "Неподдерживаемые типы для операции сложения"
            ),
        }
    }

    fn subtract_values(
        &self,
        left: Value,
        right: Value,
        span: Span,
    ) -> Result<Value, RuntimeError> {
        match (left, right) {
            (Value::Number(a), Value::Number(b)) => Ok(Value::Number(a - b)),
            (Value::Float(a), Value::Float(b)) => Ok(Value::Float(a - b)),
            (Value::Number(a), Value::Float(b)) => Ok(Value::Float((a as f64) - b)),
            (Value::Float(a), Value::Number(b)) => Ok(Value::Float(a - (b as f64))),

            _ => bail_runtime!(TypeMismatch, span, "Вычитание применимо только к числам"),
        }
    }

    fn multiply_values(
        &self,
        left: Value,
        right: Value,
        span: Span,
    ) -> Result<Value, RuntimeError> {
        match (left, right) {
            (Value::Number(a), Value::Number(b)) => Ok(Value::Number(a * b)),
            (Value::Float(a), Value::Float(b)) => Ok(Value::Float(a * b)),
            (Value::Number(a), Value::Float(b)) => Ok(Value::Float((a as f64) * b)),
            (Value::Float(a), Value::Number(b)) => Ok(Value::Float(a * (b as f64))),
            _ => bail_runtime!(TypeMismatch, span, "Умножение применимо только к числам"),
        }
    }

    fn divide_values(&self, left: Value, right: Value, span: Span) -> Result<Value, RuntimeError> {
        match (left, right) {
            (Value::Number(a), Value::Number(b)) => {
                if b == 0 {
                    bail_runtime!(DivisionByZero, span, "Деление на 0 запрещено")
                } else {
                    Ok(Value::Number(a / b))
                }
            }
            (Value::Float(a), Value::Float(b)) => {
                if b == 0.0 {
                    bail_runtime!(DivisionByZero, span, "Деление на 0 запрещено")
                } else {
                    Ok(Value::Float(a / b))
                }
            }
            (Value::Float(a), Value::Number(b)) => {
                if b == 0 {
                    bail_runtime!(DivisionByZero, span, "Деление на 0 запрещено")
                } else {
                    Ok(Value::Float(a / b as f64))
                }
            }
            (Value::Number(a), Value::Float(b)) => {
                if b == 0.0 {
                    bail_runtime!(DivisionByZero, span, "Деление на 0 запрещено")
                } else {
                    Ok(Value::Float(a as f64 / b))
                }
            }
            _ => bail_runtime!(TypeMismatch, span, "Деление применимо только к числам"),
        }
    }

    fn modulo_values(&self, left: Value, right: Value, span: Span) -> Result<Value, RuntimeError> {
        match (left, right) {
            (Value::Number(a), Value::Number(b)) => {
                if b == 0 {
                    bail_runtime!(DivisionByZero, span, "Деление на 0 запрещено")
                } else {
                    Ok(Value::Number(a % b))
                }
            }
            (Value::Float(a), Value::Float(b)) => {
                if b == 0.0 {
                    bail_runtime!(DivisionByZero, span, "Деление на 0 запрещено")
                } else {
                    Ok(Value::Float(a % b))
                }
            }
            (Value::Float(a), Value::Number(b)) => {
                if b == 0 {
                    bail_runtime!(DivisionByZero, span, "Деление на 0 запрещено")
                } else {
                    Ok(Value::Float(a % b as f64))
                }
            }
            (Value::Number(a), Value::Float(b)) => {
                if b == 0.0 {
                    bail_runtime!(DivisionByZero, span, "Деление на 0 запрещено")
                } else {
                    Ok(Value::Float(a as f64 % b))
                }
            }
            _ => bail_runtime!(
                TypeMismatch,
                span,
                "Остаток от деления применим только к числам"
            ),
        }
    }

    fn compare_greater(
        &self,
        left: Value,
        right: Value,
        span: Span,
    ) -> Result<Value, RuntimeError> {
        match (left, right) {
            (Value::Number(a), Value::Number(b)) => Ok(Value::Boolean(a > b)),
            (Value::Float(a), Value::Float(b)) => Ok(Value::Boolean(a > b)),
            (Value::Number(a), Value::Float(b)) => Ok(Value::Boolean((a as f64) > b)),
            (Value::Float(a), Value::Number(b)) => Ok(Value::Boolean(a > (b as f64))),
            _ => bail_runtime!(TypeMismatch, span, "Сравнение применимо только к числам"),
        }
    }

    fn compare_less(&self, left: Value, right: Value, span: Span) -> Result<Value, RuntimeError> {
        match (left, right) {
            (Value::Number(a), Value::Number(b)) => Ok(Value::Boolean(a < b)),
            (Value::Float(a), Value::Float(b)) => Ok(Value::Boolean(a < b)),
            (Value::Number(a), Value::Float(b)) => Ok(Value::Boolean((a as f64) < b)),
            (Value::Float(a), Value::Number(b)) => Ok(Value::Boolean(a < (b as f64))),
            _ => bail_runtime!(TypeMismatch, span, "Сравнение применимо только к числам"),
        }
    }

    fn compare_greater_equal(
        &self,
        left: Value,
        right: Value,
        span: Span,
    ) -> Result<Value, RuntimeError> {
        match (left, right) {
            (Value::Number(a), Value::Number(b)) => Ok(Value::Boolean(a >= b)),
            (Value::Float(a), Value::Float(b)) => Ok(Value::Boolean(a >= b)),
            (Value::Number(a), Value::Float(b)) => Ok(Value::Boolean((a as f64) >= b)),
            (Value::Float(a), Value::Number(b)) => Ok(Value::Boolean(a >= (b as f64))),
            _ => bail_runtime!(TypeMismatch, span, "Сравнение применимо только к числам"),
        }
    }

    fn compare_less_equal(
        &self,
        left: Value,
        right: Value,
        span: Span,
    ) -> Result<Value, RuntimeError> {
        match (left, right) {
            (Value::Number(a), Value::Number(b)) => Ok(Value::Boolean(a <= b)),
            (Value::Float(a), Value::Float(b)) => Ok(Value::Boolean(a <= b)),
            (Value::Number(a), Value::Float(b)) => Ok(Value::Boolean((a as f64) <= b)),
            (Value::Float(a), Value::Number(b)) => Ok(Value::Boolean(a <= (b as f64))),
            _ => bail_runtime!(TypeMismatch, span, "Сравнение применимо только к числам"),
        }
    }
}
