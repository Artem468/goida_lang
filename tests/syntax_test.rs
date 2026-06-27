mod common;

#[test]
fn test_english_syntax_aliases() {
    let dir = std::path::Path::new("target/english_syntax_aliases_test");
    std::fs::create_dir_all(dir).expect("Не удалось создать временную папку теста");
    let main_file = dir.join("main.goida");
    std::fs::write(
        &main_file,
        r#"
function add(a: number, b: number) -> number {
    return a + b
}

function noop() -> void {
    return void
}

value: number = 0
if (true and !false) {
    value = add(2, 3)
} else {
    value = 99
}

while (value < 7) {
    value += 1
}

for item from список(1, 2) {
    value += item
}

noop()
печать(value)
"#,
    )
    .expect("Не удалось записать временный файл");

    let output = common::goida_command()
        .args([
            "run",
            "-q",
            "-p",
            "goida-cli",
            "--",
            "run",
            main_file.to_str().unwrap(),
        ])
        .output()
        .expect("Не удалось запустить english syntax aliases test");

    assert!(
        output.status.success(),
        "english syntax aliases завершился с ошибкой\nSTDOUT: {}\nSTDERR: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!("10\n", String::from_utf8_lossy(&output.stdout));
}

#[test]
fn test_control_flow_example() {
    let output = common::goida_command()
        .args([
            "run",
            "-q",
            "-p",
            "goida-cli",
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
    let output = common::goida_command()
        .args([
            "run",
            "-q",
            "-p",
            "goida-cli",
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

    let output = common::goida_command()
        .args([
            "run",
            "-q",
            "-p",
            "goida-cli",
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

    let output = common::goida_command()
        .args([
            "run",
            "-q",
            "-p",
            "goida-cli",
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
    let output = common::goida_command()
        .args([
            "run",
            "-q",
            "-p",
            "goida-cli",
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
    let output = common::goida_command()
        .args([
            "run",
            "-q",
            "-p",
            "goida-cli",
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
    let output = common::goida_command()
        .args([
            "run",
            "-q",
            "-p",
            "goida-cli",
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
fn test_static_method_call_with_double_colon() {
    let dir = std::path::Path::new("target/static_method_double_colon_test");
    std::fs::create_dir_all(dir).expect("failed to create temporary test directory");

    let source = r#"
args = System::args()
print(1)
"#;
    let main_file = dir.join("main.goida");
    std::fs::write(&main_file, source).expect("failed to write temporary file");

    let output = common::goida_command()
        .args([
            "run",
            "-q",
            "-p",
            "goida-cli",
            "--",
            "run",
            main_file.to_str().unwrap(),
        ])
        .output()
        .expect("failed to run cargo");

    assert!(
        output.status.success(),
        "static method double colon failed\nSTDOUT: {}\nSTDERR: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!("1\n", String::from_utf8_lossy(&output.stdout));
}

#[test]
fn test_static_method_call_with_dot_is_rejected() {
    let dir = std::path::Path::new("target/static_method_dot_rejected_test");
    std::fs::create_dir_all(dir).expect("failed to create temporary test directory");

    let source = r#"
System.args()
"#;
    let main_file = dir.join("main.goida");
    std::fs::write(&main_file, source).expect("failed to write temporary file");

    let output = common::goida_command()
        .args([
            "run",
            "-q",
            "-p",
            "goida-cli",
            "--",
            "run",
            main_file.to_str().unwrap(),
        ])
        .output()
        .expect("failed to run cargo");

    assert!(
        !output.status.success(),
        "static method call with dot unexpectedly succeeded"
    );
    let diagnostics = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        diagnostics.contains("Static method must be called with '::'"),
        "unexpected diagnostics: {diagnostics}"
    );
}

#[test]
fn test_static_method_call_with_dot_uses_detected_russian_error() {
    let dir = std::path::Path::new("target/static_method_dot_russian_error_test");
    std::fs::create_dir_all(dir).expect("failed to create temporary test directory");

    let source = r#"
Система.аргументы()
"#;
    let main_file = dir.join("main.goida");
    std::fs::write(&main_file, source).expect("failed to write temporary file");

    let output = common::goida_command()
        .args([
            "run",
            "-q",
            "-p",
            "goida-cli",
            "--",
            "run",
            main_file.to_str().unwrap(),
        ])
        .output()
        .expect("failed to run cargo");

    assert!(
        !output.status.success(),
        "static method call with dot unexpectedly succeeded"
    );
    let diagnostics = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        diagnostics.contains("Статический метод нужно вызывать через '::'"),
        "unexpected diagnostics: {diagnostics}"
    );
}

#[test]
fn test_instance_method_call_with_double_colon_is_rejected() {
    let dir = std::path::Path::new("target/instance_method_double_colon_rejected_test");
    std::fs::create_dir_all(dir).expect("failed to create temporary test directory");

    let source = r#"
class Counter {
    public function value(self) -> number {
        return 1
    }
}

counter = new Counter()
counter::value()
"#;
    let main_file = dir.join("main.goida");
    std::fs::write(&main_file, source).expect("failed to write temporary file");

    let output = common::goida_command()
        .args([
            "run",
            "-q",
            "-p",
            "goida-cli",
            "--",
            "run",
            main_file.to_str().unwrap(),
        ])
        .output()
        .expect("failed to run cargo");

    assert!(
        !output.status.success(),
        "instance method call with double colon unexpectedly succeeded"
    );
    let diagnostics = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        diagnostics.contains("Instance method must be called with '.'"),
        "unexpected diagnostics: {diagnostics}"
    );
}

#[test]
fn test_runtime_errors_are_localized_from_existing_messages() {
    let dir = std::path::Path::new("target/runtime_error_localization_test");
    std::fs::create_dir_all(dir).expect("failed to create temporary test directory");

    let russian_file = dir.join("russian.goida");
    std::fs::write(
        &russian_file,
        r#"
результат = 1 / 0
"#,
    )
    .expect("failed to write russian temporary file");

    let russian_output = common::goida_command()
        .args([
            "run",
            "-q",
            "-p",
            "goida-cli",
            "--",
            "run",
            russian_file.to_str().unwrap(),
        ])
        .output()
        .expect("failed to run cargo");

    assert!(
        !russian_output.status.success(),
        "division by zero unexpectedly succeeded"
    );
    let russian_diagnostics = format!(
        "{}{}",
        String::from_utf8_lossy(&russian_output.stdout),
        String::from_utf8_lossy(&russian_output.stderr)
    );
    assert!(
        russian_diagnostics.contains("Деление на ноль"),
        "unexpected diagnostics: {russian_diagnostics}"
    );

    let english_file = dir.join("english.goida");
    std::fs::write(
        &english_file,
        r#"
list().get()
"#,
    )
    .expect("failed to write english temporary file");

    let english_output = common::goida_command()
        .args([
            "run",
            "-q",
            "-p",
            "goida-cli",
            "--",
            "run",
            english_file.to_str().unwrap(),
        ])
        .output()
        .expect("failed to run cargo");

    assert!(
        !english_output.status.success(),
        "invalid list get unexpectedly succeeded"
    );
    let english_diagnostics = format!(
        "{}{}",
        String::from_utf8_lossy(&english_output.stdout),
        String::from_utf8_lossy(&english_output.stderr)
    );
    assert!(
        english_diagnostics.contains("Usage: list.get(number)"),
        "unexpected diagnostics: {english_diagnostics}"
    );
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
Система::сон(50)
печать(значение)
"#;
    let main_file = dir.join("main.goida");
    std::fs::write(&main_file, source).expect("Не удалось записать временный файл");

    let output = common::goida_command()
        .args([
            "run",
            "-q",
            "-p",
            "goida-cli",
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
