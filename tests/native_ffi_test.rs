use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

fn dylib_name() -> &'static str {
    if cfg!(target_os = "windows") {
        "native_ffi_test_lib.dll"
    } else if cfg!(target_os = "macos") {
        "libnative_ffi_test_lib.dylib"
    } else {
        "libnative_ffi_test_lib.so"
    }
}

#[test]
fn native_library_block_loads_relative_dylib_and_exposes_exports() {
    let build = Command::new("cargo")
        .args(["build", "-p", "native_ffi_test_lib"])
        .output()
        .expect("failed to build native ffi test library");
    assert!(
        build.status.success(),
        "build failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&build.stdout),
        String::from_utf8_lossy(&build.stderr)
    );

    let script_dir = Path::new("target/native_ffi_case");
    fs::create_dir_all(script_dir).expect("failed to create script directory");

    let script = format!(
        r#"
библиотека "../debug/{}" {{
    функция add(a: число, b: число) -> число {{}}
    функция identity(value: неизвестно) -> неизвестно {{}}
    переменная counter: число;
}}

печать(add(2, 5));
печать(identity(список(1, 2, 3))[1]);
counter = 9;
печать(counter);
"#,
        dylib_name()
    );

    let script_path = script_dir.join("native_ffi.goida");
    fs::write(&script_path, script).expect("failed to write ffi script");

    let output = Command::new("cargo")
        .args(["run", "-p", "cli", "--", "run"])
        .arg(path_to_arg(&script_path))
        .output()
        .expect("failed to run cli");

    assert!(
        output.status.success(),
        "cli run failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("7"),
        "stdout did not contain add result: {stdout}"
    );
    assert!(
        stdout.contains("2"),
        "stdout did not contain identity result: {stdout}"
    );
    assert!(
        stdout.contains("9"),
        "stdout did not contain updated counter: {stdout}"
    );
}

fn path_to_arg(path: &Path) -> String {
    let path: PathBuf = path.into();
    path.to_string_lossy().into_owned()
}
