use crate::ast::prelude::{ClassDefinition, Span, Visibility};
use crate::ast::program::FieldData;
use crate::interpreter::prelude::{BuiltinFn, SharedInterner, Value};
use crate::shared::SharedMut;
use std::io::{stdin, stdout, Write};
use std::sync::Arc;
use string_interner::DefaultSymbol as Symbol;

pub fn setup_terminal_class(interner_ref: &SharedInterner) -> (Symbol, SharedMut<ClassDefinition>) {
    let name_sym = interner_ref.write(|i| i.get_or_intern("Терминал"));
    let mut class_def = ClassDefinition::new(name_sym, Span::default());

    let colors = [
        ("сброс", "\x1b[0m"),
        ("жирный", "\x1b[1m"),
        ("тусклый", "\x1b[2m"),
        ("курсив", "\x1b[3m"),
        ("подчеркнутый", "\x1b[4m"),
        ("мигающий", "\x1b[5m"),
        ("быстро_мигающий", "\x1b[6m"),
        ("инверсия", "\x1b[7m"),
        ("скрытый", "\x1b[8m"),
        ("зачеркнутый", "\x1b[9m"),

        // --- Цвета текста (Стандартные) ---
        ("черный", "\x1b[30m"),
        ("красный", "\x1b[31m"),
        ("зеленый", "\x1b[32m"),
        ("желтый", "\x1b[33m"),
        ("синий", "\x1b[34m"),
        ("пурпурный", "\x1b[35m"),
        ("циан", "\x1b[36m"),
        ("белый", "\x1b[37m"),

        // --- Цвета текста (Яркие) ---
        ("серый", "\x1b[90m"),
        ("ярко_красный", "\x1b[91m"),
        ("ярко_зеленый", "\x1b[92m"),
        ("ярко_желтый", "\x1b[93m"),
        ("ярко_синий", "\x1b[94m"),
        ("ярко_пурпурный", "\x1b[95m"),
        ("ярко_циан", "\x1b[96m"),
        ("ярко_белый", "\x1b[97m"),

        // --- Цвета фона (Стандартные) ---
        ("фон_черный", "\x1b[40m"),
        ("фон_красный", "\x1b[41m"),
        ("фон_зеленый", "\x1b[42m"),
        ("фон_желтый", "\x1b[43m"),
        ("фон_синий", "\x1b[44m"),
        ("фон_пурпурный", "\x1b[45m"),
        ("фон_циан", "\x1b[46m"),
        ("фон_белый", "\x1b[47m"),

        // --- Цвета фона (Яркие) ---
        ("фон_серый", "\x1b[100m"),
        ("фон_ярко_красный", "\x1b[101m"),
        ("фон_ярко_зеленый", "\x1b[102m"),
        ("фон_ярко_желтый", "\x1b[103m"),
        ("фон_ярко_синий", "\x1b[104m"),
        ("фон_ярко_пурпурный", "\x1b[105m"),
        ("фон_ярко_циан", "\x1b[106m"),
        ("фон_ярко_белый", "\x1b[107m"),
    ];

    for (name, code) in colors {
        let sym = interner_ref.write(|i| i.get_or_intern(name));

        class_def.fields.insert(
            sym,
            (
                Visibility::Public,
                true,
                FieldData::Value(SharedMut::new(Value::Text(code.to_string())))
            )
        );
    }

    // --- Терминал.очистить() ---
    class_def.add_method(
        interner_ref.write(|i| i.get_or_intern("очистить")),
        Visibility::Public,
        true,
        BuiltinFn(Arc::new(move |_, _, _| {
            // ANSI escape-последовательность для очистки экрана и возврата курсора в 1,1
            print!("\x1B[2J\x1B[1;1H");
            let _ = std::io::stdout().flush();
            Ok(Value::Empty)
        })),
    );

    // Метод: Терминал.заголовок(текст)
    class_def.add_method(
        interner_ref.write(|i| i.get_or_intern("заголовок")),
        Visibility::Public,
        true,
        BuiltinFn(Arc::new(move |_, args, _| {
            let title = args.get(1).map(|v| v.to_string()).unwrap_or_default();
            print!("\x1b]0;{}\x07", title);
            let _ = stdout().flush();
            Ok(Value::Empty)
        })),
    );

    // Метод: Терминал.скрыть_курсор()
    class_def.add_method(
        interner_ref.write(|i| i.get_or_intern("скрыть_курсор")),
        Visibility::Public,
        true,
        BuiltinFn(Arc::new(move |_, _, _| {
            print!("\x1b[?25l");
            let _ = stdout().flush();
            Ok(Value::Empty)
        })),
    );

    // Метод: Терминал.показать_курсор()
    class_def.add_method(
        interner_ref.write(|i| i.get_or_intern("показать_курсор")),
        Visibility::Public,
        true,
        BuiltinFn(Arc::new(move |_, _, _| {
            print!("\x1b[?25h");
            let _ = stdout().flush();
            Ok(Value::Empty)
        })),
    );

    // --- Терминал.позиция(х, у) ---
    class_def.add_method(
        interner_ref.write(|i| i.get_or_intern("позиция")),
        Visibility::Public,
        true,
        BuiltinFn(Arc::new(move |_, args, _span| {
            let x = args.get(1).and_then(|v| v.as_i64()).unwrap_or(1);
            let y = args.get(2).and_then(|v| v.as_i64()).unwrap_or(1);
            // ANSI: \x1b[Y;XH (отсчет с 1)
            print!("\x1b[{};{}H", y, x);
            let _ = stdout().flush();
            Ok(Value::Empty)
        }))
    );

    // --- Терминал.пауза(сообщение) ---
    class_def.add_method(
        interner_ref.write(|i| i.get_or_intern("пауза")),
        Visibility::Public,
        true,
        BuiltinFn(Arc::new(move |_, args, _| {
            let msg = args.get(1)
                .and_then(|v| v.as_str())
                .map(|s| s.as_str())
                .unwrap_or("Нажмите Enter, чтобы продолжить...");

            print!("{}", msg);
            let _ = stdout().flush();

            let mut buffer = String::new();
            let _ = stdin().read_line(&mut buffer);

            Ok(Value::Empty)
        }))
    );

    (name_sym, SharedMut::new(class_def))
}
