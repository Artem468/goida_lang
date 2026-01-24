mod ast;
mod interpreter;
mod macros;
mod parser;
mod traits;

use crate::parser::prelude::ParserStructs;
use clap::{Parser, Subcommand};
use interpreter::prelude::{Interpreter, RuntimeError};
use std::io::{self, Write};
use std::path::PathBuf;
use std::sync::{Arc, RwLock};
use std::{env, fs};
use string_interner::StringInterner;
use traits::prelude::CoreOperations;

#[derive(Parser)]
#[command(name = "goida")]
#[command(about = "Интерпретатор языка программирования Гойда")]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Выполнить файл с кодом
    Run {
        /// Путь к файлу с кодом
        file: String,
    },
    /// Запустить интерактивный режим (REPL)
    Repl,
}

fn main() {
    let cli = Cli::parse();

    match &cli.command {
        Some(Commands::Run { file }) => {
            if let Err(e) = run_file(file) {
                eprintln!("{}", e);
                std::process::exit(1);
            }
        }
        Some(Commands::Repl) => {
            run_repl();
        }
        None => {
            println!("Добро пожаловать в интерпретатор языка Гойда!");
            println!("Использование:");
            println!("  goida run <файл>  - выполнить файл");
            println!("  goida repl        - интерактивный режим");
            println!("  goida --help      - показать справку");
        }
    }
}

fn run_file(filename: &str) -> Result<(), Box<dyn std::error::Error>> {
    let content = fs::read_to_string(filename)
        .map_err(|e| format!("Не удалось прочитать файл '{}': {}", filename, e))?;

    execute_code(&content, filename)
}

fn run_repl() {
    println!("Интерактивный режим языка Гойда");
    println!("Введите 'выход' для завершения\n");
    loop {
        print!("гойда> ");
        io::stdout().flush().unwrap();

        let mut input = String::new();
        match io::stdin().read_line(&mut input) {
            Ok(_) => {
                let input = input.trim();

                if input == "выход" || input == "exit" {
                    println!("До свидания!");
                    break;
                }

                if input.is_empty() {
                    continue;
                }

                if let Err(e) = execute_code_with_interpreter(
                    input,
                    "main",
                    env::current_dir().expect("Не удалось получить текущую директорию"),
                ) {
                    eprintln!("Ошибка: {}", e);
                }
            }
            Err(e) => {
                eprintln!("Ошибка ввода: {}", e);
                break;
            }
        }
    }
}

fn execute_code(code: &str, filename: &str) -> Result<(), Box<dyn std::error::Error>> {
    let _path = PathBuf::from(filename);
    let _path_clone = _path.clone();
    let file_stem = _path.file_stem().and_then(|s| s.to_str()).unwrap();
    execute_code_with_interpreter(code, file_stem, _path_clone)
}

fn execute_code_with_interpreter(
    code: &str,
    filename: &str,
    path_buf: PathBuf,
) -> Result<(), Box<dyn std::error::Error>> {
    let interner = Arc::new(RwLock::new(StringInterner::new()));

    let parser = ParserStructs::Parser::new(Arc::clone(&interner), filename, path_buf);
    match parser.parse(code) {
        Ok(program) => {
            let mut interpreter = Interpreter::new(program.clone(), Arc::clone(&interner));
            interpreter.define_builtins();
            interpreter.interpret(program).map_err(|e| match e {
                RuntimeError::UndefinedVariable(name) => {
                    format!("Неопределенная переменная: {}", name)
                }
                RuntimeError::UndefinedFunction(name) => {
                    format!("Неопределенная функция: {}", name)
                }
                RuntimeError::UndefinedMethod(name) => format!("Неопределенный метод: {}", name),
                RuntimeError::TypeMismatch(msg) => format!("Несоответствие типов: {}", msg),
                RuntimeError::DivisionByZero => "Деление на ноль".to_string(),
                RuntimeError::InvalidOperation(msg) => format!("Недопустимая операция: {}", msg),
                RuntimeError::IOError(msg) => format!("Ошибка чтения файла: {}", msg),
                RuntimeError::TypeError(msg) => format!("Недопустимый тип данных: {}", msg),
                RuntimeError::Return(_) => "Неожиданный return".to_string(),
            })?;
        }
        Err(err) => eprintln!("{:#?}", err),
    }

    Ok(())
}
