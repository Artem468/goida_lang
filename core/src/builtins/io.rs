use crate::ast::prelude::ErrorData;
use crate::ast::span::Span;
use crate::interpreter::prelude::{Interpreter, RuntimeError, SharedInterner, Value};
use crate::{bail_runtime, define_builtin, runtime_error};
use std::io;
use std::io::Write;

pub fn setup_io_func(interpreter: &mut Interpreter, interner: &SharedInterner) {
    let separator = interner.write(|i| i.get_or_intern("разделитель"));
    let end = interner.write(|i| i.get_or_intern("конец"));
    let out = interner.write(|i| i.get_or_intern("файл"));

    define_builtin!(interpreter, interner, "печать" => (_interpreter, mut arguments, _span) {
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
                let file = std::fs::File::create(path).map_err(|e| {
                    runtime_error!(
                        IOError,
                        Span::default(),
                        "Ошибка вывода {}",
                        e
                    )
                })?;
                Box::new(file)
            }
        };

        let output = arguments
            .iter()
            .map(|arg| arg.value.to_string())
            .collect::<Vec<String>>()
            .join(&_sep);

        write!(writer, "{}{}", output, _end).map_err(|e| {
            runtime_error!(
                IOError,
                Span::default(),
                "Ошибка вывода {}",
                e
            )
        })?;
        writer.flush().map_err(|e| {
            runtime_error!(
                IOError,
                Span::default(),
                "Ошибка вывода {}",
                e
            )
        })?;
        Ok(Value::Empty)
    });

    define_builtin!(interpreter, interner, "ввод" => (_interpreter, arguments, span) {
        if arguments.len() != 1 {
            return bail_runtime!(
                InvalidOperation,
                span,
                "Функция 'ввод' ожидает 1 аргумент, получено {}",
                arguments.len()
            )
        }

        print!("{}", arguments[0].value);
        let _ = io::stdout().flush();

        let mut input = String::new();
        if let Ok(_) = io::stdin().read_line(&mut input) {
            Ok(Value::Text(input.trim().to_string()))
        } else {
            bail_runtime!(
                IOError,
                span,
                "Не удалось прочитать ввод"
            )
        }
    });
}
