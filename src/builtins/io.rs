use crate::ast::prelude::ErrorData;
use crate::interpreter::prelude::{BuiltinFn, Interpreter, RuntimeError, SharedInterner, Value};
use std::io;
use std::io::Write;
use std::sync::Arc;
use crate::ast::span::Span;

pub fn setup_io_func(interpreter: &mut Interpreter, interner: &SharedInterner) {
    let separator = interner.write(|i| i.get_or_intern("разделитель"));
    let end = interner.write(|i| i.get_or_intern("конец"));
    let out = interner.write(|i| i.get_or_intern("файл"));

    interpreter.builtins.insert(
        interner.write(|i| i.get_or_intern("печать")),
        BuiltinFn(Arc::new(move |_interpreter, mut arguments, _span| {
            let sep_idx = arguments.iter().position(|arg| arg.name == Some(separator));
            let _sep = match sep_idx {
                Some(idx) => arguments.remove(idx).value.to_string(),
                None => " ".to_string(),
            };

            let end_idx = arguments.iter().position(|arg| arg.name == Some(end));
            let _end = match end_idx {
                Some(idx) => arguments.remove(idx).value.to_string(),
                None => "\n".to_string(),
            };

            let out_idx = arguments.iter().position(|arg| arg.name == Some(out));
            let out_val = out_idx.map(|idx| arguments.remove(idx).value.to_string());

            let mut writer: Box<dyn Write> = match out_val.as_deref() {
                Some("ошибка") => Box::new(io::stderr()),
                Some("вывод") | None => Box::new(io::stdout()),
                Some(path) => {
                    let file = std::fs::File::create(path)
                        .map_err(|e| RuntimeError::IOError(ErrorData::new(Span::default(), format!("Ошибка вывода {}", e))))?;
                    Box::new(file)
                }
            };

            let output = arguments
                .iter()
                .map(|arg| arg.value.to_string())
                .collect::<Vec<String>>()
                .join(&_sep);

            write!(writer, "{}{}", output, _end).map_err(|e| RuntimeError::IOError(ErrorData::new(Span::default(), format!("Ошибка вывода {}", e))))?;
            writer.flush().map_err(|e| RuntimeError::IOError(ErrorData::new(Span::default(), format!("Ошибка вывода {}", e))))?;
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
            print!("{}", arguments[0].value);
            io::stdout().flush().unwrap();

            let mut input = String::new();
            io::stdin().read_line(&mut input).unwrap();
            let input = input.trim();

            Ok(Value::Text(input.to_string()))
        })),
    );
}
