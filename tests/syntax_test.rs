use std::process::Command;

#[test]
fn test_control_flow_example() {
    let output = Command::new("cargo")
        .args(["run", "-q", "-p", "cli", "--", "run", "examples/control_flow_test.goida"]) 
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
        .args(["run", "-q", "-p", "cli", "--", "run", "examples/classes_test.goida"]) 
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
fn test_queue_example() {
    let output = Command::new("cargo")
        .args(["run", "-q", "-p", "cli", "--", "run", "examples/queue_test.goida"]) 
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
        .args(["run", "-q", "-p", "cli", "--", "run", "examples/json_roundtrip_test.goida"]) 
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
