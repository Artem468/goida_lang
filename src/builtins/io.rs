use crate::ast::prelude::ErrorData;
use crate::interpreter::prelude::{BuiltinFn, Interpreter, RuntimeError, SharedInterner, Value};
use std::io;
use std::io::Write;
use std::sync::Arc;

pub fn setup_io_func(interpreter: &mut Interpreter, interner: &SharedInterner) {
    interpreter.builtins.insert(
        interner.write(|i| i.get_or_intern("печать")),
        BuiltinFn(Arc::new(move |_interpreter, arguments, _span| {
            let sep = " ";
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
            Ok(Value::Empty)
        })),
    );

    interpreter.builtins.insert(
        interner.write(|i| i.get_or_intern("ввод")),
        BuiltinFn(Arc::new(move |_interpreter, arguments, span| {
            if arguments.len() != 1 {
                return Err(RuntimeError::InvalidOperation(ErrorData::new(
                    span,
                    format!(
                        "Функция 'ввод' ожидает 1 аргумент, получено {}",
                        arguments.len()
                    ),
                )));
            }
            print!("{}", arguments[0]);
            io::stdout().flush().unwrap();

            let mut input = String::new();
            io::stdin().read_line(&mut input).unwrap();
            let input = input.trim();

            Ok(Value::Text(input.to_string()))
        })),
    );
}
