use std::process::Command;

fn run(file: &str) -> (bool, String, String) {
    let output = Command::new("cargo")
        .args(["run", "-q", "-p", "cli", "--", "run", file])
        .output()
        .expect("failed to run");

    (
        output.status.success(),
        String::from_utf8_lossy(&output.stdout).to_string(),
        String::from_utf8_lossy(&output.stderr).to_string(),
    )
}

#[test]
fn test_param_defaults_and_named_args() {
    let (ok, out, err) = run("examples/param_defaults_test.goida");
    assert!(ok, "param_defaults_test failed: {}", err);
    assert_eq!(out, "5\n7\n11\n");
}

#[test]
fn test_empty_literal() {
    let (ok, out, err) = run("examples/empty_literal_test.goida");
    assert!(ok, "empty_literal_test failed: {}", err);
    assert_eq!(out, "истина\n");
}

#[test]
fn test_unary_and_float() {
    let (ok, out, err) = run("examples/unary_float_test.goida");
    assert!(ok, "unary_float_test failed: {}", err);
    assert_eq!(out, "-5\n-3.5\n-1.5\n");
}

#[test]
fn test_assignment_type_hints() {
    let (ok, out, err) = run("examples/assignment_type_hint_test.goida");
    assert!(ok, "assignment_type_hint_test failed: {}", err);
    assert_eq!(out, "10\nx\n0\n");
}

#[test]
fn test_compound_assignment_in_statements() {
    let dir = std::path::Path::new("target/compound_assignment_test");
    std::fs::create_dir_all(dir).expect("Не удалось создать временную папку теста");

    let source = r#"
класс Счетчик {
    публичный значение: число

    публичный конструктор новый(это, значение) {
        это.значение = значение
    }
}

число_тест = 10
число_тест += 5
число_тест -= 3
число_тест *= 2
число_тест /= 4
число_тест %= 5
печать(число_тест)

объект = новый Счетчик(4)
объект.значение += 6
печать(объект.значение)

элементы = список(1, 2, 3)
элементы[1] *= 5
печать(элементы[1])

сумма = 0
для (i = 0, i < 3, i += 1) {
    сумма += i
}
печать(сумма)
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
        "compound assignment завершился с ошибкой\nSTDOUT: {}\nSTDERR: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!("1\n10\n10\n3\n", String::from_utf8_lossy(&output.stdout));
}

#[test]
fn test_foreach_statement_iterates_collections() {
    let dir = std::path::Path::new("target/foreach_statement_test");
    std::fs::create_dir_all(dir).expect("Не удалось создать временную папку теста");

    let source = r#"
сумма = 0
для элемент в список(1, 2, 3) {
    сумма += элемент
}
печать(сумма)

текст = ""
для буква в "аб" {
    текст += буква
}
печать(текст)
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
        "foreach завершился с ошибкой\nSTDOUT: {}\nSTDERR: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!("6\nаб\n", String::from_utf8_lossy(&output.stdout));
}

#[test]
fn test_constant_assignment_cannot_be_changed() {
    let dir = std::path::Path::new("target/constant_assignment_test");
    std::fs::create_dir_all(dir).expect("Не удалось создать временную папку теста");

    let main_file = dir.join("main.goida");
    std::fs::write(
        &main_file,
        "константа лимит = 3\nлимит = 4\nпечать(лимит)\n",
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
        .expect("Не удалось запустить cargo run");

    assert!(
        !output.status.success(),
        "изменение константы должно завершаться ошибкой\nSTDOUT: {}\nSTDERR: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        String::from_utf8_lossy(&output.stdout).contains("Нельзя изменить константу")
            || String::from_utf8_lossy(&output.stderr).contains("Нельзя изменить константу"),
        "ошибка должна сообщать о запрете изменения константы\nSTDOUT: {}\nSTDERR: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn test_lazy_iterator_map_filter_reduce() {
    let dir = std::path::Path::new("target/lazy_iterator_test");
    std::fs::create_dir_all(dir).expect("Не удалось создать временную папку теста");

    let source = r#"
функция удвоить(x) {
    вернуть x * 2
}

функция больше_пяти(x) {
    вернуть x > 5
}

функция сложить(a, b) {
    вернуть a + b
}

результат = список(1, 2, 3, 4).итератор().преобразовать(удвоить).отфильтровать(больше_пяти).свернуть(сложить, 0)
печать(результат)

через_цикл = 0
для x в список(1, 2, 3, 4).итератор().преобразовать(удвоить).отфильтровать(больше_пяти) {
    через_цикл += x
}
печать(через_цикл)

готовый = итератор("аб").преобразовать(строка).список()
печать(готовый.объединить("-"))
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
        "итераторы завершились с ошибкой\nSTDOUT: {}\nSTDERR: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!("14\n14\nа-б\n", String::from_utf8_lossy(&output.stdout));
}

#[test]
fn test_string_utilities_and_regular_expressions() {
    let dir = std::path::Path::new("target/string_regex_test");
    std::fs::create_dir_all(dir).expect("Не удалось создать временную папку теста");

    let source = r##"
текст = "  abc-123 def-45  "
чистый = текст.обрезать()
печать(чистый.начинается_с("abc"))
печать(чистый.заканчивается_на("45"))
"##;
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
        "строковые утилиты завершились с ошибкой\nSTDOUT: {}\nSTDERR: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        "истина\nистина\n",
        String::from_utf8_lossy(&output.stdout)
    );
}

#[test]
fn test_foreach_array_dict_and_constant_compound_assignment() {
    let dir = std::path::Path::new("target/foreach_constant_extended_test");
    std::fs::create_dir_all(dir).expect("Не удалось создать временную папку теста");

    let ok_source = r#"
сумма = 0
для x в массив(2, 4, 6) {
    сумма += x
}
печать(сумма)

ключи = ""
для ключ в словарь("b", 2, "a", 1) {
    ключи += ключ
}
печать(ключи)
"#;
    let ok_file = dir.join("ok.goida");
    std::fs::write(&ok_file, ok_source).expect("Не удалось записать временный файл");

    let ok_output = Command::new("cargo")
        .args([
            "run",
            "-q",
            "-p",
            "cli",
            "--",
            "run",
            ok_file.to_str().unwrap(),
        ])
        .output()
        .expect("Не удалось запустить cargo run");

    assert!(
        ok_output.status.success(),
        "foreach по массиву/словарю завершился с ошибкой\nSTDOUT: {}\nSTDERR: {}",
        String::from_utf8_lossy(&ok_output.stdout),
        String::from_utf8_lossy(&ok_output.stderr)
    );
    assert_eq!("12\nab\n", String::from_utf8_lossy(&ok_output.stdout));

    let fail_file = dir.join("fail.goida");
    std::fs::write(&fail_file, "константа x = 1\nx += 1\n")
        .expect("Не удалось записать временный файл");
    let fail_output = Command::new("cargo")
        .args([
            "run",
            "-q",
            "-p",
            "cli",
            "--",
            "run",
            fail_file.to_str().unwrap(),
        ])
        .output()
        .expect("Не удалось запустить cargo run");

    assert!(
        !fail_output.status.success(),
        "составное изменение константы должно завершаться ошибкой\nSTDOUT: {}\nSTDERR: {}",
        String::from_utf8_lossy(&fail_output.stdout),
        String::from_utf8_lossy(&fail_output.stderr)
    );
    assert!(
        String::from_utf8_lossy(&fail_output.stdout).contains("Нельзя изменить константу")
            || String::from_utf8_lossy(&fail_output.stderr).contains("Нельзя изменить константу"),
        "ошибка должна сообщать о запрете изменения константы\nSTDOUT: {}\nSTDERR: {}",
        String::from_utf8_lossy(&fail_output.stdout),
        String::from_utf8_lossy(&fail_output.stderr)
    );
}
