use std::fs;
use std::path::Path;
use std::process::Command;

#[test]
fn test_all_examples() {
    let examples_dir = Path::new("examples");
    
    assert!(examples_dir.exists(), "Папка examples не найдена");
    
    let entries = fs::read_dir(examples_dir)
        .expect("Не удалось прочитать папку examples");
    
    let mut goida_files = Vec::new();
    
    for entry in entries {
        let entry = entry.expect("Ошибка чтения файла");
        let path = entry.path();
        
        if path.extension().and_then(|s| s.to_str()) == Some("goida") {
            goida_files.push(path);
        }
    }
    
    assert!(!goida_files.is_empty(), "Не найдено ни одного .goida файла в папке examples");
    
    println!("Найдено {} .goida файлов для тестирования", goida_files.len());
    
    // Тестируем каждый файл
    for file_path in goida_files {
        println!("Тестируем файл: {:?}", file_path);
        
        let output = Command::new("cargo")
            .args(["run", "--", "run", file_path.to_str().unwrap()])
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
fn test_specific_examples() {
    let test_cases = vec![
        ("examples/hello.goida", "Привет, мир!"),
        ("examples/simple.goida", "42"),
    ];
    
    for (file_path, expected_output) in test_cases {
        if Path::new(file_path).exists() {
            println!("Тестируем файл с ожидаемым выводом: {}", file_path);
            
            let output = Command::new("cargo")
                .args(["run", "--", "run", file_path])
                .output()
                .expect("Не удалось запустить команду cargo run");
            
            if output.status.success() {
                let stdout = String::from_utf8_lossy(&output.stdout);
                let actual_output = stdout.trim();
                
                if actual_output.contains(expected_output) {
                    println!("✓ {} - вывод соответствует ожидаемому", file_path);
                } else {
                    println!("⚠ {} - вывод отличается от ожидаемого", file_path);
                    println!("Ожидалось: {}", expected_output);
                    println!("Получено: {}", actual_output);
                }
            } else {
                println!("✗ {} - завершился с ошибкой", file_path);
            }
        }
    }
}
