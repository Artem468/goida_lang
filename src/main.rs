mod lexer;
mod ast;
mod parser;
mod interpreter;

use clap::{Parser, Subcommand};
use std::{env, fs};
use std::io::{self, Write};
use std::path::PathBuf;
use lexer::prelude::LexerStructs::Lexer;
use parser::prelude::ParserStructs::{Parser as GoidaParser, ParseError};
use interpreter::prelude::InterpreterStructs::{Interpreter, RuntimeError};
use interpreter::prelude::InterpreterTraits::{CoreOperations};

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
    let mut interpreter = Interpreter::new(env::current_dir().expect("Не удалось получить текущую директорию"));
    
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

                if let Err(e) = execute_code_with_interpreter(&mut interpreter, input, "main") {
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
    let _path = std::path::PathBuf::from(filename);
    let file_stem = _path.file_stem().and_then(|s| s.to_str()).ok_or_else(|| {
        RuntimeError::InvalidOperation(format!(
            "Невозможно получить имя модуля из пути: {}",
            _path.display()
        ))
    }).unwrap();
    let mut interpreter = Interpreter::new(PathBuf::from(_path.parent().unwrap()));
    execute_code_with_interpreter(&mut interpreter, code, file_stem)
}

fn execute_code_with_interpreter(interpreter: &mut Interpreter, code: &str, filename: &str) -> Result<(), Box<dyn std::error::Error>> {
    let mut lexer = Lexer::new(code.to_string());
    let tokens = lexer.tokenize();

    let parser = GoidaParser::new(tokens, filename.to_string());
    let program = parser.parse().map_err(|e| match e {
        ParseError::UnexpectedToken(msg) => format!("Синтаксическая ошибка: {}", msg),
        ParseError::InternalError(msg) => format!("Внутренняя ошибка парсера: {}", msg),
        ParseError::TypeError(msg) => format!("Ошибка типа: {}", msg),
    })?;
    
    interpreter.interpret(program).map_err(|e| match e {
        RuntimeError::UndefinedVariable(name) => format!("Неопределенная переменная: {}", name),
        RuntimeError::UndefinedFunction(name) => format!("Неопределенная функция: {}", name),
        RuntimeError::TypeMismatch(msg) => format!("Несоответствие типов: {}", msg),
        RuntimeError::DivisionByZero => "Деление на ноль".to_string(),
        RuntimeError::InvalidOperation(msg) => format!("Недопустимая операция: {}", msg),
        RuntimeError::IOError(msg) => format!("Ошибка чтения файла: {}", msg),
        RuntimeError::ParseError(msg) => format!("Ошибка при парсинге: {}", msg),
        RuntimeError::Return(_) => "Неожиданный return".to_string(),
    })?;

    Ok(())
}
