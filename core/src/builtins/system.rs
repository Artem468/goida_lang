use crate::ast::prelude::{ClassDefinition, ErrorData, Span};
use crate::builtins::registry::*;
use crate::interpreter::prelude::{CallArgListExt, RuntimeError, SharedInterner, Value};
use crate::shared::SharedMut;
use crate::{bail_runtime, define_method, runtime_error};
use std::io::Write;
use std::sync::Arc;
use string_interner::DefaultSymbol as Symbol;

pub fn setup_system_class(interner_ref: &SharedInterner) -> (Symbol, SharedMut<ClassDefinition>) {
    let name = interner_ref.write(|i| i.get_or_intern(class::SYSTEM.names.canonical));
    let mut class_def = ClassDefinition::new(name, Span::default());

    // --- Система.выход(код) ---
    define_method!(class_def, interner_ref, @static method::EXIT.canonical => (_, args, _) {
        let code = match CallArgListExt::first_value(&args) {
            Some(Value::Number(n)) => *n as i32,
            _ => 0,
        };
        std::process::exit(code);
    });

    // --- Система.паника(сообщение) ---
    define_method!(class_def, interner_ref, @static method::PANIC.canonical => (interpreter, args, span) {
        let msg = CallArgListExt::first_value(&args)
            .map(|v| interpreter.format_value(v))
            .unwrap_or_else(|| "Неизвестная ошибка".into());
        bail_runtime!(
            Panic,
            span,
            "{}", msg
        )

    });

    // --- Система.платформа() -> Text ---
    define_method!(class_def, interner_ref, @static method::PLATFORM.canonical => (_, _, _) {
        let os = std::env::consts::OS; // "windows", "linux", "macos"
        Ok(Value::Text(os.to_string()))
    });

    // --- Система.аргументы() -> List ---
    define_method!(class_def, interner_ref, @static method::ARGS.canonical => (_, _, _) {
        let args_os: Vec<Value> = std::env::args()
            .skip_while(|arg| arg != "--")
            .skip(1)
            .map(Value::Text)
            .collect();

        Ok(Value::Array(Arc::new(args_os)))
    });

    // --- Система.время() -> Number (мс) ---
    define_method!(class_def, interner_ref, @static method::TIME.canonical => (_, _, _) {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as i64;
        Ok(Value::Number(now))
    });

    // --- Система.сон(миллисекунды) ---
    define_method!(class_def, interner_ref, @static method::SLEEP.canonical => (_, args, span) {
        let ms = match CallArgListExt::first_value(&args) {
            Some(Value::Number(n)) => *n,
            _ => {
                return bail_runtime!(
                    TypeError,
                    span,
                    "Функция 'сон' ожидает число (миллисекунды)"
                )
            }
        };

        if ms < 0 {
            return bail_runtime!(
                InvalidOperation,
                span,
                "Функция 'сон' ожидает число (миллисекунды)"
            );
        }

        std::thread::sleep(std::time::Duration::from_millis(ms as u64));

        Ok(Value::Empty)
    });

    // --- Система.сигнал() ---
    define_method!(class_def, interner_ref, @static method::BEEP.canonical => (_, _, _) {
        print!("\x07");
        let _ = std::io::stdout().flush();
        Ok(Value::Empty)
    });

    // --- Система.окружение("SOME") ---
    define_method!(class_def, interner_ref, @static method::ENV.canonical => (interpreter, args, span) {
        let arg = CallArgListExt::first_value(&args)
            .map(|v| interpreter.format_value(v))
            .unwrap_or_else(|| "Неизвестная ошибка".into());
        match std::env::var(arg) {
            Ok(v) => Ok(Value::Text(v)),
            Err(err) => {
                    bail_runtime!(
                    InvalidOperation,
                    span,
                    "{}", err.to_string()
                )
            }
        }
    });

    (name, SharedMut::new(class_def))
}
