use std::process::Command;

fn run(file: &str) -> (bool, String, String) {
    let output = Command::new("cargo")
        .args(["run", "-q", "-p", "goida-cli", "--", "run", file])
        .output()
        .expect("failed to run");

    (
        output.status.success(),
        String::from_utf8_lossy(&output.stdout).to_string(),
        String::from_utf8_lossy(&output.stderr).to_string(),
    )
}

#[test]
fn test_imports_modules() {
    let (ok, out, err) = run("examples/imports_test.goida");
    assert!(ok, "imports_test failed: {}", err);
    assert_eq!(out, "123\n15\n");
}

#[test]
fn test_builtin_classes_and_methods() {
    let (ok, out, err) = run("examples/builtin_classes_test.goida");
    assert!(ok, "builtin_classes_test failed: {}", err);
    // ожидаем: истина, 3, 1
    assert!(out.contains("истина"));
    assert!(out.contains("3"));
    assert!(out.contains("1"));
}

#[test]
fn test_boundary_values() {
    let (ok, out, err) = run("examples/boundary_values_test.goida");
    assert!(ok, "boundary_values_test failed: {}", err);
    assert!(out.contains("0"));
    assert!(out.contains("-1"));
    assert!(out.contains("9223372036854775807"));
}
