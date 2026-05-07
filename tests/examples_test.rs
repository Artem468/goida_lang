use std::fs;
use std::path::Path;
use std::process::Command;

#[test]
fn test_all_examples() {
    let examples_dir = Path::new("examples");

    assert!(examples_dir.exists(), "Папка examples не найдена");

    let entries = fs::read_dir(examples_dir).expect("Не удалось прочитать папку examples");

    let mut goida_files = Vec::new();

    for entry in entries {
        let entry = entry.expect("Ошибка чтения файла");
        let path = entry.path();

        if path.extension().and_then(|s| s.to_str()) == Some("goida") {
            goida_files.push(path);
        }
    }

    assert!(
        !goida_files.is_empty(),
        "Не найдено ни одного .goida файла в папке examples"
    );

    println!(
        "Найдено {} .goida файлов для тестирования",
        goida_files.len()
    );

    for file_path in goida_files {
        println!("Тестируем файл: {:?}", file_path);

        let output = Command::new("cargo")
            .args(["run", "-p", "cli", "--", "run", file_path.to_str().unwrap()])
            .output()
            .expect("Не удалось запустить команду cargo run");

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            let stdout = String::from_utf8_lossy(&output.stdout);

            eprintln!("ОШИБКА в файле {:?}:", file_path);
            eprintln!("STDOUT: {}", stdout);
            eprintln!("STDERR: {}", stderr);
            eprintln!("Код выхода: {:?}", output.status.code());

            panic!("Файл {:?} завершился с ошибкой", file_path);
        } else {
            println!("✓ {:?} - успешно выполнен", file_path);
        }
    }

    println!("Все примеры успешно протестированы!");
}

#[test]
fn test_imported_top_level_globals_are_available() {
    let output = Command::new("cargo")
        .args([
            "run",
            "-q",
            "-p",
            "cli",
            "--",
            "run",
            "examples/import_globals.goida",
        ])
        .output()
        .expect("Не удалось запустить команду cargo run");

    assert!(
        output.status.success(),
        "import_globals.goida завершился с ошибкой\nSTDOUT: {}\nSTDERR: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    assert_eq!("42\nok\n", String::from_utf8_lossy(&output.stdout));
}

#[test]
fn test_local_binding_shadows_import_alias_for_property_access() {
    let dir = Path::new("target/import_shadow_property_access");
    fs::create_dir_all(dir).expect("Не удалось создать временную папку теста");
    fs::write(dir.join("mod.goida"), "value = 41;\n")
        .expect("Не удалось записать временный модуль");
    fs::write(
        dir.join("main.goida"),
        "подключить \"mod\" в m;\nm = \"shadowed\";\nпечать(m.value);\n",
    )
    .expect("Не удалось записать временный основной файл");

    let main_file = dir.join("main.goida");
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
        .expect("Не удалось запустить команду cargo run");

    assert!(
        !output.status.success(),
        "локальная переменная должна затенять alias импорта\nSTDOUT: {}\nSTDERR: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}
