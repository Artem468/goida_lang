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
