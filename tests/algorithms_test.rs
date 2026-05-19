use std::process::Command;

#[test]
fn test_stack_algorithm() {
    let output = Command::new("cargo")
        .args(["run", "-q", "-p", "cli", "--", "run", "examples/stack_test.goida"]) 
        .output()
        .expect("Не удалось запустить stack_test.goida");

    assert!(
        output.status.success(),
        "stack_test.goida завершился с ошибкой\nSTDOUT: {}\nSTDERR: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    assert_eq!("20\n10\n0\n", String::from_utf8_lossy(&output.stdout));
}

#[test]
fn test_sort_algorithm() {
    let output = Command::new("cargo")
        .args(["run", "-q", "-p", "cli", "--", "run", "examples/sort_test.goida"]) 
        .output()
        .expect("Не удалось запустить sort_test.goida");

    assert!(
        output.status.success(),
        "sort_test.goida завершился с ошибкой\nSTDOUT: {}\nSTDERR: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    assert_eq!("1\n2\n3\n", String::from_utf8_lossy(&output.stdout));
}

#[test]
fn test_filter_map_algorithm() {
    let output = Command::new("cargo")
        .args(["run", "-q", "-p", "cli", "--", "run", "examples/filter_test.goida"]) 
        .output()
        .expect("Не удалось запустить filter_test.goida");

    assert!(
        output.status.success(),
        "filter_test.goida завершился с ошибкой\nSTDOUT: {}\nSTDERR: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    assert_eq!("6\n8\n", String::from_utf8_lossy(&output.stdout));
}

#[test]
fn test_recursion_factorial() {
    let output = Command::new("cargo")
        .args(["run", "-q", "-p", "cli", "--", "run", "examples/recursion_test.goida"]) 
        .output()
        .expect("Не удалось запустить recursion_test.goida");

    assert!(
        output.status.success(),
        "recursion_test.goida завершился с ошибкой\nSTDOUT: {}\nSTDERR: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    assert_eq!("120\n", String::from_utf8_lossy(&output.stdout));
}
