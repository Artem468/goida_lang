use crate::ast::prelude::{ErrorData, Span};
use crate::interpreter::prelude::{BuiltinFn, Interpreter, RuntimeError, Value};
use crate::traits::prelude::CoreOperations;
use crate::{define_builtin, setup_builtins};
use std::collections::HashMap;
use std::io;
use std::io::Write;
use std::sync::{Arc, RwLock};

setup_builtins!(interpreter, {
    "печать" (arguments, span) {
        let sep =  " ";
        let end = "\n";
        print!(
            "{}{}",
            arguments
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<String>>()
                .join(sep),
            end
        );
    }

    "ввод" (arguments, span) -> Result<Value, RuntimeError> {
        if arguments.len() != 1 {
            return Err(RuntimeError::InvalidOperation(ErrorData::new(
                span,
                format!(
                    "Функция 'ввод' ожидает 1 аргумент, получено {}",
                    arguments.len()
                )
            )));
        }
        print!("{}", arguments[0]);
        io::stdout().flush().unwrap();

        let mut input = String::new();
        io::stdin().read_line(&mut input).unwrap();
        let input = input.trim();

        Ok(Value::Text(input.to_string()))
    }

     "число" (arguments, span) -> Result<Value, RuntimeError> {
        if arguments.len() != 1 {
            return Err(RuntimeError::InvalidOperation(ErrorData::new(
                span,
                format!(
                    "Функция 'число' ожидает 1 аргумент, получено {}",
                    arguments.len()
                )
            )));
        }
        let n: i64 = match arguments[0].clone().try_into() {
            Ok(i) => i,
            Err(err) => return Err(RuntimeError::InvalidOperation(ErrorData::new(span, err)))
        };
        Ok(Value::Number(n))
    }

    "дробь" (arguments, span) -> Result<Value, RuntimeError> {
        if arguments.len() != 1 {
            return Err(RuntimeError::InvalidOperation(ErrorData::new(
                span,
                format!(
                    "Функция 'дробь' ожидает 1 аргумент, получено {}",
                    arguments.len()
                )
            )));
        }
        let n: f64 = match arguments[0].clone().try_into() {
            Ok(i) => i,
            Err(err) => return Err(RuntimeError::InvalidOperation(ErrorData::new(span, err)))
        };
        Ok(Value::Float(n))
    }

    "строка" (arguments, span) -> Result<Value, RuntimeError> {
        if arguments.len() != 1 {
            return Err(RuntimeError::InvalidOperation(ErrorData::new(
                span,
                format!(
                    "Функция 'дробь' ожидает 1 аргумент, получено {}",
                    arguments.len()
                )
            )));
        }
        let n: String = arguments[0].clone().try_into()?;
        Ok(Value::Text(n))
    }

    "логический" (arguments, span) -> Result<Value, RuntimeError> {
        if arguments.len() != 1 {
            return Err(RuntimeError::InvalidOperation(ErrorData::new(
                span,
                format!(
                    "Функция 'дробь' ожидает 1 аргумент, получено {}",
                    arguments.len()
                )
            )));
        }
        let n: bool = arguments[0].clone().try_into()?;
        Ok(Value::Boolean(n))
    }

    "тип" (arguments, span) -> Result<Value, RuntimeError> {
        if arguments.len() != 1 {
            return Err(RuntimeError::InvalidOperation(ErrorData::new(
                span,
                format!(
                    "Функция 'тип' ожидает 1 аргумент, получено {}",
                    arguments.len()
                )
            )));
        }
        let val = arguments.get(0).ok_or_else(|| RuntimeError::InvalidOperation(
            ErrorData::new(
                span,
                "Не передан объект".into()
            )
        ))?;
        match val
         {
            Value::Number(_) => Ok(Value::Text("число".to_string())),
            Value::Float(_) => Ok(Value::Text("дробь".to_string())),
            Value::Text(_) => Ok(Value::Text("строка".to_string())),
            Value::Boolean(_) => Ok(Value::Text("логический".to_string())),
            Value::Object(obj) => Ok(
                Value::Text(
                     format!(
                         "объект \"{}\"",
                         interpreter
                         .resolve_symbol(obj.read().map_err(|_| {
                        RuntimeError::Panic(ErrorData::new(
                            Span::default(),
                            "Сбой блокировки в реализации функции 'тип'".into(),
                    ))
                })?.class_name)
                         .ok_or_else(|| RuntimeError::InvalidOperation(ErrorData::new(
                            span,
                            "Тип не найден".into())))?.to_string()
                    )
                 )
            ),
            Value::Class(cls) => Ok(
                Value::Text(
                    format!(
                        "класс \"{}\"",
                        interpreter
                            .resolve_symbol(cls.read().map_err(|_| {
                        RuntimeError::Panic(ErrorData::new(
                            Span::default(),
                            "Сбой блокировки в реализации функции 'тип'".into(),
                    ))
                })?.name)
                            .ok_or_else(|| RuntimeError::InvalidOperation(ErrorData::new(
                                span,
                                "Тип не найден".into())))?.to_string()
                    )
                )
            ),
            Value::Function(obj) => Ok(
                Value::Text(
                    format!(
                         "функция \"{}\"",
                         interpreter
                        .resolve_symbol(obj.name)
                        .ok_or_else(|| RuntimeError::InvalidOperation(ErrorData::new(
                            span,
                            "Тип не найден".into())))?.to_string()
                    )
                )
            ),
            Value::Builtin(_) => {
                Ok(Value::Text("встроенная функция".to_string()))
            },
            Value::Module(sym) => {
                Ok(Value::Text(
                    format!(
                        "модуль \"{}\"",
                        interpreter
                        .resolve_symbol(*sym)
                        .ok_or_else(|| RuntimeError::InvalidOperation(ErrorData::new(
                            span,
                            "Модуль не найден".into())))?
                    )
                ))
            },
            Value::List(_) => {
                Ok(Value::Text("список".to_string()))
            },
            Value::Array(_) => {
                Ok(Value::Text("массив".to_string()))
            },
            Value::Dict(_) => {
                Ok(Value::Text("словарь".to_string()))
            },
            Value::NativeResource(_) => {
                Ok(Value::Text("ресурс".to_string()))
            }
            Value::Empty => {
                Ok(Value::Text("пустота".to_string()))
            }
        }
    }

    "список" (arguments, span) -> Result<Value, RuntimeError> {
        Ok(Value::List(Arc::new(RwLock::new(arguments))))
    }

    "массив" (arguments, span) -> Result<Value, RuntimeError> {
        Ok(Value::Array(Arc::new(arguments)))
    }

    "словарь" (arguments, span) -> Result<Value, RuntimeError> {
        if arguments.len() % 2 != 0 {
            return Err(RuntimeError::InvalidOperation(ErrorData::new(
                span,
                "Функция 'словарь' ожидает четное количество аргументов (пары ключ-значение)".to_string()
            )));
        }

        let mut dict = HashMap::new();
        for i in (0..arguments.len()).step_by(2) {
            let key = match &arguments[i] {
                Value::Text(s) => s.clone(),
                v => v.to_string(),
            };
            let value = arguments[i + 1].clone();
            dict.insert(key, value);
        }

        Ok(Value::Dict(Arc::new(RwLock::new(dict))))
    }
});
