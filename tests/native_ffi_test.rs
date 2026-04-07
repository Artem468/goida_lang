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
    функция add_f64(a: дробь, b: дробь) -> дробь {{}}
    функция identity_ptr(value: указатель) -> указатель {{}}
    функция make_ptr() -> указатель {{}}
    переменная counter: число;
    переменная ratio: дробь;
    переменная handle: указатель;
}}

печать(add(2, 5));
печать(add_f64(1.25, 2.5));
печать(identity_ptr(make_ptr()));
печать(identity_ptr("строка"));
печать(identity_ptr(список(1, 2, 3))[1]);
печать(identity_ptr(массив(10, 20, 30))[1]);
печать(identity_ptr(словарь("ключ", 42))["ключ"]);
counter = 9;
ratio = 2.25;
handle = make_ptr();
печать(counter);
печать(ratio);
печать(handle);
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
        "stdout missing number result: {stdout}"
    );
    assert!(
        stdout.contains("3.75"),
        "stdout missing float result: {stdout}"
    );
    assert!(
        stdout.contains("4660"),
        "stdout missing pointer result: {stdout}"
    );
    assert!(
        stdout.contains("строка"),
        "stdout missing text roundtrip: {stdout}"
    );
    assert!(
        stdout.contains("2"),
        "stdout missing list roundtrip: {stdout}"
    );
    assert!(
        stdout.contains("20"),
        "stdout missing array roundtrip: {stdout}"
    );
    assert!(
        stdout.contains("42"),
        "stdout missing dict roundtrip: {stdout}"
    );
    assert!(
        stdout.contains("9"),
        "stdout missing counter result: {stdout}"
    );
    assert!(
        stdout.contains("2.25"),
        "stdout missing ratio result: {stdout}"
    );
}

fn path_to_arg(path: &Path) -> String {
    let path: PathBuf = path.into();
    path.to_string_lossy().into_owned()
}
