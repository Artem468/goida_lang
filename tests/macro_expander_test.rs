use std::process::Command;

fn run_source(name: &str, source: &str) -> (bool, String, String) {
    let dir = std::path::Path::new("target").join(name);
    std::fs::create_dir_all(&dir).expect("failed to create test directory");
    let file = dir.join("main.goida");
    std::fs::write(&file, source).expect("failed to write source file");

    let output = Command::new("cargo")
        .args([
            "run",
            "-q",
            "-p",
            "cli",
            "--",
            "run",
            file.to_str().unwrap(),
        ])
        .output()
        .expect("failed to run cli");

    (
        output.status.success(),
        String::from_utf8_lossy(&output.stdout).to_string(),
        String::from_utf8_lossy(&output.stderr).to_string(),
    )
}

#[test]
fn test_expression_macro_expands_before_ast_build() {
    let source = r#"
macro twice {
    ($x:expr) => { $x + $x };
}

печать(twice!(3))
печать(twice![4])
печать(twice!{5})
"#;

    let (ok, out, err) = run_source("macro_expression_test", source);
    assert!(ok, "expression macro failed\nSTDOUT: {out}\nSTDERR: {err}");
    assert_eq!("6\n8\n10\n", out);
}

#[test]
fn test_statement_macro_call_replaces_whole_statement() {
    let source = r#"
макрос лог {
    ($значение:выр) => { печать($значение) };
}

лог!(5)
"#;

    let (ok, out, err) = run_source("macro_statement_test", source);
    assert!(ok, "statement macro failed\nSTDOUT: {out}\nSTDERR: {err}");
    assert_eq!("5\n", out);
}

#[test]
fn test_repetition_macro_expands_comma_separated_exprs() {
    let source = r#"
macro make_list {
    ($( $x:expr ),*) => { список($( $x ),*) };
}

печать(make_list!(1, 2, 3).объединить(","))
"#;

    let (ok, out, err) = run_source("macro_repetition_test", source);
    assert!(ok, "repetition macro failed\nSTDOUT: {out}\nSTDERR: {err}");
    assert_eq!("1,2,3\n", out);
}

#[test]
fn test_statement_fragment_macro_can_generate_block() {
    let source = r#"
macro when {
    ($condition:expr, $statement:stmt) => { if ($condition) { $statement } };
}

value = 0
when!(true, value = 7)
печать(value)
"#;

    let (ok, out, err) = run_source("macro_statement_fragment_test", source);
    assert!(
        ok,
        "statement fragment macro failed\nSTDOUT: {out}\nSTDERR: {err}"
    );
    assert_eq!("7\n", out);
}

#[test]
fn test_builtin_format_macro_formats_values() {
    let source = r#"
имя = "Анна"
возраст = 21
печать(format!("Привет, {}, тебе {} лет", имя, возраст))
печать(формат!("Имя: {}", имя))
печать(format!("{{}} = {}", 42))
"#;

    let (ok, out, err) = run_source("macro_builtin_format_test", source);
    assert!(ok, "format macro failed\nSTDOUT: {out}\nSTDERR: {err}");
    assert_eq!("Привет, Анна, тебе 21 лет\nИмя: Анна\n{} = 42\n", out);
}

#[test]
fn test_macro_with_multiple_match_rules_uses_first_matching_rule() {
    let source = r#"
macro choose {
    () => { "empty" };
    ($value:expr) => { $value };
}

печать(choose!())
печать(choose!("value"))
"#;

    let (ok, out, err) = run_source("macro_multiple_rules_test", source);
    assert!(ok, "multi-rule macro failed\nSTDOUT: {out}\nSTDERR: {err}");
    assert_eq!("empty\nvalue\n", out);
}
