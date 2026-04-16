use crate::ast::prelude::{ClassDefinition, ErrorData, Span};
use crate::define_method;
use crate::interpreter::prelude::{CallArgListExt, RuntimeError, SharedInterner, Value};
use crate::shared::SharedMut;
use std::io::Write;
use std::sync::Arc;
use string_interner::DefaultSymbol as Symbol;

pub fn setup_system_class(interner_ref: &SharedInterner) -> (Symbol, SharedMut<ClassDefinition>) {
    let name = interner_ref.write(|i| i.get_or_intern("Система"));
    let mut class_def = ClassDefinition::new(name, Span::default());

    // --- Система.выход(код) ---
    define_method!(class_def, interner_ref, @static "выход" => (_, args, _) {
        let code = match CallArgListExt::first_value(&args) {
            Some(Value::Number(n)) => *n as i32,
            _ => 0,
        };
        std::process::exit(code);
    });

    // --- Система.паника(сообщение) ---
    define_method!(class_def, interner_ref, @static "паника" => (_, args, span) {
        let msg = CallArgListExt::get_value(&args, 1)
            .map(|v| v.to_string())
            .unwrap_or_else(|| "Неизвестная ошибка".into());
        Err(RuntimeError::Panic(ErrorData::new(span, msg)))
    });

    // --- Система.платформа() -> Text ---
    define_method!(class_def, interner_ref, @static "платформа" => (_, _, _) {
        let os = std::env::consts::OS; // "windows", "linux", "macos"
        Ok(Value::Text(os.to_string()))
    });

    // --- Система.аргументы() -> List ---
    define_method!(class_def, interner_ref, @static "аргументы" => (_, _, _) {
        let args_os: Vec<Value> = std::env::args()
            .skip_while(|arg| arg != "--")
            .skip(1)
            .map(Value::Text)
            .collect();

        Ok(Value::Array(Arc::new(args_os)))
    });

    // --- Система.время() -> Number (мс) ---
    define_method!(class_def, interner_ref, @static "время" => (_, _, _) {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as i64;
        Ok(Value::Number(now))
    });

    // --- Система.сон(миллисекунды) ---
    define_method!(class_def, interner_ref, @static "сон" => (_, args, span) {
        let ms = match CallArgListExt::get_value(&args, 1) {
            Some(Value::Number(n)) => *n,
            _ => {
                return Err(RuntimeError::TypeError(ErrorData::new(
                    span,
                    "Функция 'сон' ожидает число (миллисекунды)".into(),
                )))
            }
        };

        if ms < 0 {
            return Err(RuntimeError::InvalidOperation(ErrorData::new(
                span,
                "Время сна не может быть отрицательным".into(),
            )));
        }

        std::thread::sleep(std::time::Duration::from_millis(ms as u64));

        Ok(Value::Empty)
    });

    // --- Система.сигнал() ---
    define_method!(class_def, interner_ref, @static "сигнал" => (_, _, _) {
        print!("\x07");
        let _ = std::io::stdout().flush();
        Ok(Value::Empty)
    });

    (name, SharedMut::new(class_def))
}
