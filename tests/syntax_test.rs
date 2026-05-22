use std::process::Command;

#[test]
fn test_control_flow_example() {
    let output = Command::new("cargo")
        .args([
            "run",
            "-q",
            "-p",
            "cli",
            "--",
            "run",
            "examples/control_flow_test.goida",
        ])
        .output()
        .expect("Не удалось запустить control_flow_test.goida");

    assert!(
        output.status.success(),
        "control_flow_test.goida завершился с ошибкой\nSTDOUT: {}\nSTDERR: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    assert_eq!(
        "start\nif_ok\n0\n1\n2\n10\n11\nend\n",
        String::from_utf8_lossy(&output.stdout)
    );
}

#[test]
fn test_classes_example() {
    let output = Command::new("cargo")
        .args([
            "run",
            "-q",
            "-p",
            "cli",
            "--",
            "run",
            "examples/classes_test.goida",
        ])
        .output()
        .expect("Не удалось запустить classes_test.goida");

    assert!(
        output.status.success(),
        "classes_test.goida завершился с ошибкой\nSTDOUT: {}\nSTDERR: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    assert_eq!("7\n14\n", String::from_utf8_lossy(&output.stdout));
}

#[test]
fn test_method_chain_can_continue_on_new_lines() {
    let dir = std::path::Path::new("target/multiline_method_chain_test");
    std::fs::create_dir_all(dir).expect("Не удалось создать временную папку теста");
    let main_file = dir.join("main.goida");
    std::fs::write(
        &main_file,
        r#"
класс Запрос {
    публичный значение: число = 0

    публичный функция ссылка(это, _url: строка) {
        это.значение += 1
        вернуть это
    }

    публичный функция заголовки(это, _headers: словарь) {
        это.значение += 10
        вернуть это
    }

    публичный функция ожидание(это, _timeout: число) {
        это.значение += 100
        вернуть это
    }
}

класс Сессия {
    публичный функция запрос(это) {
        вернуть новый Запрос()
    }
}

сес = новый Сессия()
зап = сес.запрос()
    .ссылка("https://example.com")
    .заголовки(словарь("Content-Type", "application/json"))
    .ожидание(30)

печать(зап.значение)
"#,
    )
    .expect("Не удалось записать временный файл");

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
        .expect("Не удалось запустить multiline method chain test");

    assert!(
        output.status.success(),
        "multiline method chain завершился с ошибкой\nSTDOUT: {}\nSTDERR: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!("111\n", String::from_utf8_lossy(&output.stdout));
}

#[test]
fn test_binary_expressions_can_continue_on_new_lines() {
    let dir = std::path::Path::new("target/multiline_binary_expression_test");
    std::fs::create_dir_all(dir).expect("Не удалось создать временную папку теста");
    let main_file = dir.join("main.goida");
    std::fs::write(
        &main_file,
        r#"
сумма = 1
    + 2
    * 3
    - 4

условие = сумма
    >= 3
    и сумма
    < 10
    или ложь

печать(сумма)
печать(условие)
"#,
    )
    .expect("Не удалось записать временный файл");

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
        .expect("Не удалось запустить multiline binary expression test");

    assert!(
        output.status.success(),
        "multiline binary expression завершился с ошибкой\nSTDOUT: {}\nSTDERR: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!("3\nистина\n", String::from_utf8_lossy(&output.stdout));
}

#[test]
fn test_queue_example() {
    let output = Command::new("cargo")
        .args([
            "run",
            "-q",
            "-p",
            "cli",
            "--",
            "run",
            "examples/queue_test.goida",
        ])
        .output()
        .expect("Не удалось запустить queue_test.goida");

    assert!(
        output.status.success(),
        "queue_test.goida завершился с ошибкой\nSTDOUT: {}\nSTDERR: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    assert_eq!("1\n2\n0\n", String::from_utf8_lossy(&output.stdout));
}

#[test]
fn test_json_roundtrip_example() {
    let output = Command::new("cargo")
        .args([
            "run",
            "-q",
            "-p",
            "cli",
            "--",
            "run",
            "examples/json_roundtrip_test.goida",
        ])
        .output()
        .expect("Не удалось запустить json_roundtrip_test.goida");

    assert!(
        output.status.success(),
        "json_roundtrip_test.goida завершился с ошибкой\nSTDOUT: {}\nSTDERR: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    assert_eq!("1\nx\n", String::from_utf8_lossy(&output.stdout));
}

#[test]
fn test_thread_example() {
    let output = Command::new("cargo")
        .args([
            "run",
            "-q",
            "-p",
            "cli",
            "--",
            "run",
            "examples/thread_test.goida",
        ])
        .output()
        .expect("Не удалось запустить thread_test.goida");

    assert!(
        output.status.success(),
        "thread_test.goida завершился с ошибкой\nSTDOUT: {}\nSTDERR: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    assert_eq!("2\n3\n", String::from_utf8_lossy(&output.stdout));
}

#[test]
fn test_top_level_thread_block_executes_and_updates_outer_variable() {
    let dir = std::path::Path::new("target/top_level_thread_block_test");
    std::fs::create_dir_all(dir).expect("Не удалось создать временную папку теста");

    let source = r#"
значение = 1
поток {
    значение = 2
}
Система.сон(50)
печать(значение)
"#;
    let main_file = dir.join("main.goida");
    std::fs::write(&main_file, source).expect("Не удалось записать временный файл");

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
        "top-level thread block завершился с ошибкой\nSTDOUT: {}\nSTDERR: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!("2\n", String::from_utf8_lossy(&output.stdout));
}
