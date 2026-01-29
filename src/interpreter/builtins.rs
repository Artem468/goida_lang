use crate::Arc;
use crate::interpreter::prelude::{BuiltinFn, Interpreter, RuntimeError, Value};
use crate::traits::prelude::CoreOperations;
use crate::{define_builtin, setup_builtins};
use std::io;
use std::io::Write;


setup_builtins!(interpreter, {
    "печать" (arguments) {
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

    "ввод" (arguments) -> Result<Value, RuntimeError> {
        if arguments.len() != 1 {
            return Err(RuntimeError::InvalidOperation(format!(
                "Функция 'ввод' ожидает 1 аргумент, получено {}",
                arguments.len()
            )));
        }
        print!("{}", arguments[0]);
        io::stdout().flush().unwrap();

        let mut input = String::new();
        io::stdin().read_line(&mut input).unwrap();
        let input = input.trim();

        Ok(Value::Text(input.to_string()))
    }

     "число" (arguments) -> Result<Value, RuntimeError> {
        if arguments.len() != 1 {
            return Err(RuntimeError::InvalidOperation(format!(
                "Функция 'число' ожидает 1 аргумент, получено {}",
                arguments.len()
            )));
        }
        let n: i64 = arguments[0].clone().try_into()?;
        Ok(Value::Number(n))
    }

    "дробь" (arguments) -> Result<Value, RuntimeError> {
        if arguments.len() != 1 {
            return Err(RuntimeError::InvalidOperation(format!(
                "Функция 'дробь' ожидает 1 аргумент, получено {}",
                arguments.len()
            )));
        }
        let n: f64 = arguments[0].clone().try_into()?;
        Ok(Value::Float(n))
    }

    "текст" (arguments) -> Result<Value, RuntimeError> {
        if arguments.len() != 1 {
            return Err(RuntimeError::InvalidOperation(format!(
                "Функция 'дробь' ожидает 1 аргумент, получено {}",
                arguments.len()
            )));
        }
        let n: String = arguments[0].clone().try_into()?;
        Ok(Value::Text(n))
    }

    "логический" (arguments) -> Result<Value, RuntimeError> {
        if arguments.len() != 1 {
            return Err(RuntimeError::InvalidOperation(format!(
                "Функция 'дробь' ожидает 1 аргумент, получено {}",
                arguments.len()
            )));
        }
        let n: bool = arguments[0].clone().try_into()?;
        Ok(Value::Boolean(n))
    }

    "тип" (arguments) -> Result<Value, RuntimeError> {
        if arguments.len() != 1 {
            return Err(RuntimeError::InvalidOperation(format!(
                "Функция 'тип' ожидает 1 аргумент, получено {}",
                arguments.len()
            )));
        }
        let val = arguments.get(0).ok_or_else(|| RuntimeError::InvalidOperation("Не передан объект".into()))?;
        match val
         {
            Value::Number(_) => Ok(Value::Text("число".to_string())),
            Value::Float(_) => Ok(Value::Text("дробь".to_string())),
            Value::Text(_) => Ok(Value::Text("текст".to_string())),
            Value::Boolean(_) => Ok(Value::Text("логический".to_string())),
            Value::Object(obj) => Ok(
                Value::Text(
                     format!(
                         "объект \"{}\"",
                         interpreter
                         .resolve_symbol(obj.borrow().class_name)
                         .ok_or_else(|| RuntimeError::InvalidOperation("Тип не найден".into()))?.to_string()
                     )
                 )

            ),
            Value::Function(obj) => Ok(
                Value::Text(
                    format!(
                         "функция \"{}\"",
                         interpreter
                        .resolve_symbol(obj.name)
                        .ok_or_else(|| RuntimeError::InvalidOperation("Тип не найден".into()))?.to_string()
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
                        .ok_or_else(|| RuntimeError::InvalidOperation("Модуль не найден".into()))?
                    )
                ))
            },
            Value::Empty => {
                Ok(Value::Text("пустота".to_string()))
            }
        }
    }
});
