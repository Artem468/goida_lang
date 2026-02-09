use std::io::Write;
use crate::ast::prelude::{ClassDefinition, ErrorData, Span, Visibility};
use crate::interpreter::prelude::{BuiltinFn, RuntimeError, SharedInterner, Value};
use crate::shared::SharedMut;
use std::sync::Arc;
use string_interner::DefaultSymbol as Symbol;

pub fn setup_system_class(interner_ref: &SharedInterner) -> (Symbol, SharedMut<ClassDefinition>) {
    let name = interner_ref.write(|i| i.get_or_intern("Система"));
    let mut class_def = ClassDefinition::new(name, Span::default());

    // --- Система.выход(код) ---
    class_def.add_method(
        interner_ref.write(|i| i.get_or_intern("выход")),
        Visibility::Public,
        true, // Делаем статическим, если ваш движок это поддерживает
        BuiltinFn(Arc::new(move |_, args, _| {
            let code = match args.get(0) {
                Some(Value::Number(n)) => *n as i32,
                _ => 0,
            };
            std::process::exit(code);
        })),
    );

    // --- Система.паника(сообщение) ---
    class_def.add_method(
        interner_ref.write(|i| i.get_or_intern("паника")),
        Visibility::Public,
        true,
        BuiltinFn(Arc::new(move |_, args, span| {
            let msg = args.get(1).map(|v| v.to_string()).unwrap_or_else(|| "Неизвестная ошибка".into());
            Err(RuntimeError::Panic(ErrorData::new(span, msg)))
        })),
    );

    // --- Система.платформа() -> Text ---
    class_def.add_method(
        interner_ref.write(|i| i.get_or_intern("платформа")),
        Visibility::Public,
        true,
        BuiltinFn(Arc::new(move |_, _, _| {
            let os = std::env::consts::OS; // "windows", "linux", "macos"
            Ok(Value::Text(os.to_string()))
        })),
    );

    // --- Система.аргументы() -> List ---
    class_def.add_method(
        interner_ref.write(|i| i.get_or_intern("аргументы")),
        Visibility::Public,
        true,
        BuiltinFn(Arc::new(move |_, _, _| {
            let args_os: Vec<Value> = std::env::args()
                // Пропускаем всё ДО разделителя "--" и сам разделитель
                .skip_while(|arg| arg != "--")
                .skip(1)
                .map(Value::Text)
                .collect();

            Ok(Value::Array(Arc::new(args_os)))
        })),
    );

    // --- Система.время() -> Number (мс) ---
    class_def.add_method(
        interner_ref.write(|i| i.get_or_intern("время")),
        Visibility::Public,
        true,
        BuiltinFn(Arc::new(move |_, _, _| {
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as i64;
            Ok(Value::Number(now))
        })),
    );

    // --- Система.сон(миллисекунды) ---
    class_def.add_method(
        interner_ref.write(|i| i.get_or_intern("сон")),
        Visibility::Public,
        true,
        BuiltinFn(Arc::new(move |_, args, span| {
            let ms = match args.get(1) {
                Some(Value::Number(n)) => *n,
                _ => return Err(RuntimeError::TypeError(ErrorData::new(
                    span,
                    "Функция 'сон' ожидает число (миллисекунды)".into()
                ))),
            };

            if ms < 0 {
                return Err(RuntimeError::InvalidOperation(ErrorData::new(
                    span,
                    "Время сна не может быть отрицательным".into()
                )));
            }

            std::thread::sleep(std::time::Duration::from_millis(ms as u64));

            Ok(Value::Empty)
        })),
    );

    // --- Система.сигнал() ---
    class_def.add_method(
        interner_ref.write(|i| i.get_or_intern("сигнал")),
        Visibility::Public,
        true,
        BuiltinFn(Arc::new(move |_, _, _| {
            // ASCII Bell character (звуковой сигнал терминала)
            print!("\x07");
            let _ = std::io::stdout().flush();
            Ok(Value::Empty)
        })),
    );

    // --- Система.нано_время() -> Number (наносекунды) ---
    class_def.add_method(
        interner_ref.write(|i| i.get_or_intern("нано_время")),
        Visibility::Public,
        true,
        BuiltinFn(Arc::new(move |_, _, _| {
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default();

            Ok(Value::Number(now.as_nanos() as i64))
        })),
    );


    (name, SharedMut::new(class_def))
}
