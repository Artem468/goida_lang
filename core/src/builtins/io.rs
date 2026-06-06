use crate::ast::prelude::ErrorData;
use crate::ast::span::Span;
use crate::interpreter::prelude::{Interpreter, RuntimeError, SharedInterner, Value};
use crate::{bail_runtime, define_builtin, expect_args, runtime_error};
use std::io;
use std::io::Write;

pub fn setup_io_func(interpreter: &mut Interpreter, interner: &SharedInterner) {
    let separators =
        ["разделитель", "sep", "separator"].map(|name| interner.write(|i| i.get_or_intern(name)));
    let ends = ["конец", "end"].map(|name| interner.write(|i| i.get_or_intern(name)));
    let outs = ["файл", "file"].map(|name| interner.write(|i| i.get_or_intern(name)));

    define_builtin!(interpreter, interner, crate::builtins::catalog::function::PRINT.canonical => (interpreter, mut arguments, _span) {
        let sep_idx = arguments
            .iter()
            .position(|arg| arg.name.is_some_and(|name| separators.contains(&name)));
        let _sep = match sep_idx {
            Some(idx) => interpreter.format_value(&arguments.remove(idx).value),
            None => " ".to_string(),
        };

        let end_idx = arguments
            .iter()
            .position(|arg| arg.name.is_some_and(|name| ends.contains(&name)));
        let _end = match end_idx {
            Some(idx) => interpreter.format_value(&arguments.remove(idx).value),
            None => "\n".to_string(),
        };

        let out_idx = arguments
            .iter()
            .position(|arg| arg.name.is_some_and(|name| outs.contains(&name)));
        let out_val = out_idx.map(|idx| interpreter.format_value(&arguments.remove(idx).value));

        let mut writer: Box<dyn Write> = match out_val.as_deref() {
            Some("ошибка") | Some("stderr") => Box::new(io::stderr()),
            Some("вывод") | Some("stdout") | None => Box::new(io::stdout()),
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
            .map(|arg| interpreter.format_value(&arg.value))
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

    define_builtin!(interpreter, interner, crate::builtins::catalog::function::INPUT.canonical => (interpreter, arguments, span) {
        expect_args!(arguments, 1, span, "ввод");

        print!("{}", interpreter.format_value(&arguments[0].value));
        let _ = io::stdout().flush();

        let mut input = String::new();
        if io::stdin().read_line(&mut input).is_ok() {
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
