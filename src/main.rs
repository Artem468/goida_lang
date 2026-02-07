mod ast;
mod builtins;
mod interpreter;
mod parser;
mod shared;
mod traits;

use ariadne::{Color, Label, Report, ReportKind};
use ast::prelude::{ErrorData, Span};
use clap::{Parser, Subcommand};
use interpreter::prelude::{Interpreter, RuntimeError};
use lazy_static::lazy_static;
use parser::prelude::{ParseError, Parser as ProgramParser};
use std::io::{self, Write};
use std::path::PathBuf;
use std::sync::RwLock;
use std::{env, fs};
use traits::prelude::CoreOperations;

lazy_static! {
    static ref INTERPRETER: RwLock<Interpreter> = RwLock::new(Interpreter::new());
}

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
            if let Err(_) = run_file(file) {
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

fn run_file(filename: &str) -> Result<(), (String, ErrorData)> {
    let content = fs::read_to_string(filename).map_err(|e| {
        (
            format!("Не удалось прочитать файл '{}': {}", filename, e),
            ErrorData::new(
                Span::default(),
                format!("Не удалось прочитать файл '{}': {}", filename, e),
            ),
        )
    })?;

    match execute_code(&content, filename) {
        Ok(_) => Ok(()),
        Err((msg, error)) => {
            let _res = Err((msg.clone(), error.clone()));
            let file_name = INTERPRETER
                .read()
                .unwrap()
                .get_file_path(&error.location.file_id);

            let file_code = INTERPRETER
                .read()
                .unwrap()
                .source_manager
                .get_file_content(file_name.as_str());
            let ariadne_span = error.location.as_ariadne(file_code.as_str());
            Report::build(ReportKind::Error, (&file_name, ariadne_span.clone()))
                .with_message(msg)
                .with_label(
                    Label::new((&file_name, ariadne_span))
                        .with_message(error.message)
                        .with_color(Color::Red),
                )
                .finish()
                .print(&INTERPRETER.read().unwrap().source_manager)
                .expect("Can't build report message");
            _res
        }
    }
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
                    eprintln!("Ошибка: {}", e.0);
                }
            }
            Err(e) => {
                eprintln!("Ошибка ввода: {}", e);
                break;
            }
        }
    }
}

fn execute_code(code: &str, filename: &str) -> Result<(), (String, ErrorData)> {
    let _path = PathBuf::from(filename);
    let _path_clone = _path.clone();
    let file_stem = _path.file_stem().and_then(|s| s.to_str()).unwrap();
    execute_code_with_interpreter(code, file_stem, _path_clone)
}

fn execute_code_with_interpreter(
    code: &str,
    filename: &str,
    path_buf: PathBuf,
) -> Result<(), (String, ErrorData)> {
    let parser = ProgramParser::new(
        INTERPRETER.read().unwrap().interner.clone(),
        filename,
        path_buf,
    );
    // Пустой модуль для вывода данных о нем на случай ошибки
    let _module = parser.module.clone();

    match parser.parse(code) {
        Ok(program) => {
            let _name = program.name;
            INTERPRETER.write().unwrap().load_start_module(program);
            INTERPRETER.write().unwrap().define_builtins();
            INTERPRETER
                .write()
                .unwrap()
                .interpret(_name)
                .map_err(|e| match e {
                    RuntimeError::UndefinedVariable(err) => {
                        (format!("Неопределенная переменная: {}", err.message), err)
                    }
                    RuntimeError::UndefinedFunction(err) => {
                        (format!("Неопределенная функция: {}", err.message), err)
                    }
                    RuntimeError::UndefinedMethod(err) => {
                        (format!("Неопределенный метод: {}", err.message), err)
                    }
                    RuntimeError::TypeMismatch(err) => {
                        (format!("Несоответствие типов: {}", err.message), err)
                    }
                    RuntimeError::Panic(err) => (format!("Паника: {}", err.message), err),
                    RuntimeError::DivisionByZero(err) => ("Деление на ноль".to_string(), err),
                    RuntimeError::InvalidOperation(err) => {
                        (format!("Недопустимая операция: {}", err.message), err)
                    }
                    RuntimeError::IOError(err) => {
                        (format!("Ошибка чтения файла: {}", err.message), err)
                    }
                    RuntimeError::TypeError(err) => {
                        (format!("Недопустимый тип данных: {}", err.message), err)
                    }
                    RuntimeError::Return(err, ..) => ("Неожиданный return".to_string(), err),
                    RuntimeError::ImportError(err) => {
                        let (msg, error) = match err {
                            ParseError::UnexpectedToken(e) => ("Неожиданный токен", e),
                            ParseError::TypeError(e) => ("Ошибка типов", e),
                            ParseError::InvalidSyntax(e) => ("Ошибка синтаксиса".into(), e),
                        };
                        (msg.to_string(), error)
                    }
                })?;
        }
        Err(err) => {
            INTERPRETER.write().unwrap().modules.insert(_module.name, _module);
            return match err {
                ParseError::UnexpectedToken(e) => Err(("Неожиданный токен".into(), e)),
                ParseError::TypeError(e) => Err(("Ошибка типов".into(), e)),
                ParseError::InvalidSyntax(e) => Err(("Ошибка синтаксиса".into(), e)),
            }
        }
    }

    Ok(())
}
