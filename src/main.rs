mod lexer;
mod ast;
mod parser;
mod interpreter;

use clap::{Parser, Subcommand};
use std::fs;
use std::io::{self, Write};

use lexer::Lexer;
use parser::{Parser as GoidaParser, ParseError};
use interpreter::{Interpreter, RuntimeError};

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
                eprintln!("Ошибка: {}", e);
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

    execute_code(&content)
}

fn run_repl() {
    println!("Интерактивный режим языка Гойда");
    println!("Введите 'выход' для завершения\n");

    let mut interpreter = Interpreter::new();

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

                if let Err(e) = execute_code_with_interpreter(&mut interpreter, input) {
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

fn execute_code(code: &str) -> Result<(), Box<dyn std::error::Error>> {
    let mut interpreter = Interpreter::new();
    execute_code_with_interpreter(&mut interpreter, code)
}

fn execute_code_with_interpreter(interpreter: &mut Interpreter, code: &str) -> Result<(), Box<dyn std::error::Error>> {
    let mut lexer = Lexer::new(code.to_string());
    let tokens = lexer.tokenize();
    
    let mut parser = GoidaParser::new(tokens);
    let program = parser.parse().map_err(|e| match e {
        ParseError::UnexpectedToken(msg) => format!("Синтаксическая ошибка: {}", msg),
    })?;
    
    interpreter.interpret(program).map_err(|e| match e {
        RuntimeError::UndefinedVariable(name) => format!("Неопределенная переменная: {}", name),
        RuntimeError::UndefinedFunction(name) => format!("Неопределенная функция: {}", name),
        RuntimeError::TypeMismatch(msg) => format!("Несоответствие типов: {}", msg),
        RuntimeError::DivisionByZero => "Деление на ноль".to_string(),
        RuntimeError::InvalidOperation(msg) => format!("Недопустимая операция: {}", msg),
        RuntimeError::Return(_) => "Неожиданный return".to_string(),
    })?;

    Ok(())
}
