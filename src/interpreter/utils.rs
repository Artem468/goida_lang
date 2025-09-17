use std::io;
use std::io::Write;
use crate::interpreter::structs::{Interpreter, RuntimeError, Value};
use crate::interpreter::traits::InterpreterUtils;

impl InterpreterUtils for Interpreter {
    fn input_function(&self, argument: Value) -> Result<Value, RuntimeError> {
        print!("{}", argument);
        io::stdout().flush().unwrap();

        let mut input = String::new();
        io::stdin().read_line(&mut input).unwrap();
        let input = input.trim();

        if let Ok(num) = input.parse::<i64>() {
            Ok(Value::Number(num))
        } else {
            Ok(Value::Text(input.to_string()))
        }
    }
}