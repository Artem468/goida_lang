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
            "goida-cli",
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
fn english_builtin_functions_classes_methods_and_macros_work() {
    let source = r#"
print("a", "b", sep="-", end="!\n")

items = list(1, 2)
items.push(3)
print(items.length())
print(items.join(":"))
print(items.get(1))

created = new List(4, 5)
created.push(6)
print(created.length())
print(created.join(","))

text = string("  Hi ")
print(text.trim().lower())

pattern = regex("[a-z]+")
print(pattern.matches("abc"))

first = items.get(0)
print(format!("{} {}", "ok", first))
"#;

    let (ok, out, err) = run_source("english_builtin_aliases_test", source);
    assert!(ok, "english aliases failed\nSTDOUT: {out}\nSTDERR: {err}");
    assert_eq!("a-b!\n3\n1:2:3\n2\n3\n4,5,6\nhi\nистина\nok 1\n", out);
}

#[test]
fn russian_and_english_builtin_names_can_be_mixed() {
    let source = r#"
данные = list(1, 2)
данные.добавить(3)
print(данные.объединить("-"))
размер = данные.длина()
третий = данные.get(2)
печать(format!("len={}", размер))
печать(формат!("get={}", третий))
"#;

    let (ok, out, err) = run_source("mixed_builtin_aliases_test", source);
    assert!(ok, "mixed aliases failed\nSTDOUT: {out}\nSTDERR: {err}");
    assert_eq!("1-2-3\nlen=3\nget=3\n", out);
}
