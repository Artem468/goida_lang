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
