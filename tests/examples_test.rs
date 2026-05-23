use std::fs;
use std::path::Path;
use std::process::Command;

#[test]
fn test_all_examples() {
    let examples_dir = Path::new("examples");

    assert!(examples_dir.exists(), "Папка examples не найдена");

    let entries = fs::read_dir(examples_dir).expect("Не удалось прочитать папку examples");

    let mut goida_files = Vec::new();

    for entry in entries {
        let entry = entry.expect("Ошибка чтения файла");
        let path = entry.path();

        if path.extension().and_then(|s| s.to_str()) == Some("goida") {
            goida_files.push(path);
        }
    }

    assert!(
        !goida_files.is_empty(),
        "Не найдено ни одного .goida файла в папке examples"
    );

    println!(
        "Найдено {} .goida файлов для тестирования",
        goida_files.len()
    );

    for file_path in goida_files {
        println!("Тестируем файл: {:?}", file_path);

        let output = Command::new("cargo")
            .args(["run", "-p", "cli", "--", "run", file_path.to_str().unwrap()])
            .output()
            .expect("Не удалось запустить команду cargo run");

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            let stdout = String::from_utf8_lossy(&output.stdout);

            eprintln!("ОШИБКА в файле {:?}:", file_path);
            eprintln!("STDOUT: {}", stdout);
            eprintln!("STDERR: {}", stderr);
            eprintln!("Код выхода: {:?}", output.status.code());

            panic!("Файл {:?} завершился с ошибкой", file_path);
        } else {
            println!("✓ {:?} - успешно выполнен", file_path);
        }
    }

    println!("Все примеры успешно протестированы!");
}

#[test]
fn test_imported_top_level_globals_are_available() {
    let output = Command::new("cargo")
        .args([
            "run",
            "-q",
            "-p",
            "cli",
            "--",
            "run",
            "examples/import_globals.goida",
        ])
        .output()
        .expect("Не удалось запустить команду cargo run");

    assert!(
        output.status.success(),
        "import_globals.goida завершился с ошибкой\nSTDOUT: {}\nSTDERR: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    assert_eq!("42\nok\n", String::from_utf8_lossy(&output.stdout));
}

#[test]
fn test_local_binding_shadows_import_alias_for_property_access() {
    let dir = Path::new("target/import_shadow_property_access");
    fs::create_dir_all(dir).expect("Не удалось создать временную папку теста");
    fs::write(dir.join("mod.goida"), "value = 41\n").expect("Не удалось записать временный модуль");
    fs::write(
        dir.join("main.goida"),
        "подключить \"mod\" как m\nm = \"shadowed\"\nпечать(m.value)\n",
    )
    .expect("Не удалось записать временный основной файл");

    let main_file = dir.join("main.goida");
    let output = Command::new("cargo")
        .args([
            "run",
            "-q",
            "-p",
            "cli",
            "--",
            "run",
            main_file.to_str().unwrap(),
        ])
        .output()
        .expect("Не удалось запустить команду cargo run");

    assert!(
        !output.status.success(),
        "локальная переменная должна затенять alias импорта\nSTDOUT: {}\nSTDERR: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn test_class_inheritance_reuses_base_members() {
    let dir = Path::new("target/class_inheritance_test");
    fs::create_dir_all(dir).expect("Не удалось создать временную папку теста");
    let source = r#"
класс База {
    публичный значение: число = 7

    публичный функция получить(это) -> число {
        вернуть это.значение
    }
}

класс Потомок(База) {
    публичный функция удвоить(это) -> число {
        вернуть это.получить() * 2
    }
}

объект = новый Потомок()
печать(объект.получить())
печать(объект.удвоить())
"#;
    let main_file = dir.join("main.goida");
    fs::write(&main_file, source).expect("Не удалось записать временный файл");

    let output = Command::new("cargo")
        .args([
            "run",
            "-q",
            "-p",
            "cli",
            "--",
            "run",
            main_file.to_str().unwrap(),
        ])
        .output()
        .expect("Не удалось запустить cargo run");

    assert!(
        output.status.success(),
        "наследование завершилось с ошибкой\nSTDOUT: {}\nSTDERR: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!("7\n14\n", String::from_utf8_lossy(&output.stdout));
}

#[test]
fn test_try_catch_catches_by_base_error_class() {
    let dir = Path::new("target/try_catch_test");
    fs::create_dir_all(dir).expect("Не удалось создать временную папку теста");
    let source = r#"
класс Ошибка {
}

класс ОшибкаДеленияНаНоль(Ошибка) {
}

попробовать {
    печать(10 / 0)
} перехватить (Ошибка) {
    печать("поймано")
}
"#;
    let main_file = dir.join("main.goida");
    fs::write(&main_file, source).expect("Не удалось записать временный файл");

    let output = Command::new("cargo")
        .args([
            "run",
            "-q",
            "-p",
            "cli",
            "--",
            "run",
            main_file.to_str().unwrap(),
        ])
        .output()
        .expect("Не удалось запустить cargo run");

    assert!(
        output.status.success(),
        "try/expect завершился с ошибкой\nSTDOUT: {}\nSTDERR: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!("поймано\n", String::from_utf8_lossy(&output.stdout));
}

#[test]
fn test_raise_can_be_caught_by_base_class() {
    let dir = Path::new("target/raise_test");
    fs::create_dir_all(dir).expect("Не удалось создать временную папку теста");
    let source = r#"
класс Ошибка {
}

класс МояОшибка(Ошибка) {
}

попробовать {
    выбросить МояОшибка("ручная ошибка")
} перехватить (Ошибка) {
    печать("перехвачено")
}
"#;
    let main_file = dir.join("main.goida");
    fs::write(&main_file, source).expect("Не удалось записать временный файл");

    let output = Command::new("cargo")
        .args([
            "run",
            "-q",
            "-p",
            "cli",
            "--",
            "run",
            main_file.to_str().unwrap(),
        ])
        .output()
        .expect("Не удалось запустить cargo run");

    assert!(
        output.status.success(),
        "raise/перехватить завершился с ошибкой\nSTDOUT: {}\nSTDERR: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!("перехвачено\n", String::from_utf8_lossy(&output.stdout));
}

#[test]
fn test_builtin_error_classes_do_not_need_declarations() {
    let dir = Path::new("target/builtin_error_classes_test");
    fs::create_dir_all(dir).expect("Не удалось создать временную папку теста");
    let source = r#"
попробовать {
    печать(10 / 0)
} перехватить (ОшибкаДеленияНаНоль) {
    печать("деление")
}

попробовать {
    выбросить ОшибкаОперации("ручная ошибка")
} перехватить (Ошибка) {
    печать("база")
}
"#;
    let main_file = dir.join("main.goida");
    fs::write(&main_file, source).expect("Не удалось записать временный файл");

    let output = Command::new("cargo")
        .args([
            "run",
            "-q",
            "-p",
            "cli",
            "--",
            "run",
            main_file.to_str().unwrap(),
        ])
        .output()
        .expect("Не удалось запустить cargo run");

    assert!(
        output.status.success(),
        "встроенные классы ошибок должны быть доступны без объявления\nSTDOUT: {}\nSTDERR: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!("деление\nбаза\n", String::from_utf8_lossy(&output.stdout));
}

#[test]
fn test_catch_can_receive_error_text_and_try_multiple_handlers() {
    let dir = Path::new("target/catch_error_text_test");
    fs::create_dir_all(dir).expect("Не удалось создать временную папку теста");
    let source = r#"
попробовать {
    выбросить ОшибкаОперации("важный текст")
} перехватить (ОшибкаДеленияНаНоль как сообщение) {
    печать("не должно выполниться: " + сообщение)
} перехватить (ОшибкаОперации как сообщение) {
    печать("поймано: " + сообщение)
}

попробовать {
    выбросить ОшибкаТипа("любой тип")
} перехватить (как сообщение) {
    печать("любой: " + сообщение)
}
"#;
    let main_file = dir.join("main.goida");
    fs::write(&main_file, source).expect("Не удалось записать временный файл");

    let output = Command::new("cargo")
        .args([
            "run",
            "-q",
            "-p",
            "cli",
            "--",
            "run",
            main_file.to_str().unwrap(),
        ])
        .output()
        .expect("Не удалось запустить cargo run");

    assert!(
        output.status.success(),
        "перехватчик должен получать текст ошибки\nSTDOUT: {}\nSTDERR: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        "поймано: важный текст\nлюбой: любой тип\n",
        String::from_utf8_lossy(&output.stdout)
    );
}

#[test]
fn test_unknown_variable_is_reported_while_parsing() {
    let dir = Path::new("target/parse_unknown_name_test");
    fs::create_dir_all(dir).expect("Не удалось создать временную папку теста");
    let main_file = dir.join("main.goida");
    fs::write(&main_file, "печать(не_существует)\n").expect("Не удалось записать временный файл");

    let output = Command::new("cargo")
        .args([
            "run",
            "-q",
            "-p",
            "cli",
            "--",
            "run",
            main_file.to_str().unwrap(),
        ])
        .output()
        .expect("Не удалось запустить cargo run");

    assert!(
        !output.status.success(),
        "неизвестное имя должно падать на этапе парсинга\nSTDOUT: {}\nSTDERR: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        String::from_utf8_lossy(&output.stdout).contains("Имя 'не_существует' не найдено")
            || String::from_utf8_lossy(&output.stderr).contains("Имя 'не_существует' не найдено"),
        "сообщение должно указывать на неизвестное имя\nSTDOUT: {}\nSTDERR: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}
