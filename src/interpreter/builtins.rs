use crate::interpreter::prelude::{Interpreter, RuntimeError, Value};
use crate::{define_builtin, setup_builtins};
use std::io;
use std::io::Write;

setup_builtins!(
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
);
