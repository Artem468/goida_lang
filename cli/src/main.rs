use ariadne::{Color, Label, Report, ReportKind};
use clap::{Parser, Subcommand};
use std::{
    fs,
    io::{self, Write},
    path::PathBuf,
};

use goida_core::ast::prelude::{ErrorData, Span};
use goida_core::interpreter::prelude::RuntimeError;
use goida_core::parser::prelude::{ParseError, Parser as ProgramParser};
use goida_core::traits::prelude::CoreOperations;
use goida_core::INTERPRETER;

#[derive(Parser)]
#[command(name = "goida", about = "Интерпретатор языка программирования Гойда")]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    Run {
        file: String,
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        script_args: Vec<String>,
    },
    Repl,
}

fn main() {
    let cli = Cli::parse();
    match &cli.command {
        Some(Commands::Run { file, .. }) => match run_file(file) {
            Err((err, _)) => {
                println!("{}", err);
                std::process::exit(1);
            }
            _ => {}
        },
        Some(Commands::Repl) => run_repl(),
        None => {
            println!("Добро пожаловать в Гойда! Используйте --help для справки.");
        }
    }
}

fn run_file(filename: &str) -> Result<(), (String, ErrorData)> {
    let content = fs::read_to_string(filename).map_err(|e| {
        let msg = format!("{}: '{}'", e, filename);
        (msg.clone(), ErrorData::new(Span::default(), msg))
    })?;
    execute_code(&content, filename)
}

fn execute_code(code: &str, filename: &str) -> Result<(), (String, ErrorData)> {
    let path = PathBuf::from(filename);

    let parser = ProgramParser::new(
        INTERPRETER.read().unwrap().interner.clone(),
        filename,
        path.clone(),
    );
    let _module = parser.module.clone();

    match parser.parse(code) {
        Ok(program) => {
            let name = program.name;
            {
                let mut intp = INTERPRETER.write().unwrap();
                intp.load_start_module(program);
                intp.define_builtins();
            }

            let interpret_result = {
                let mut interpreter = INTERPRETER.write().unwrap();
                interpreter.interpret(name)
            };

            interpret_result.map_err(|e| {
                let (msg, error_data) = match e {
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
                    RuntimeError::ImportError(err) => match err {
                        ParseError::TypeError(e) => ("Ошибка типов".to_string(), e),
                        ParseError::InvalidSyntax(e) => ("Ошибка синтаксиса".to_string(), e),
                        ParseError::ImportError(e) => ("Ошибка импорта".to_string(), e),
                    },
                };
                render_error(&msg, &error_data);
                (msg, error_data)
            })?;
        }
        Err(err) => {
            INTERPRETER
                .write()
                .unwrap()
                .modules
                .insert(_module.name, _module);
            let (msg, data): (&'static str, ErrorData) = match err {
                ParseError::TypeError(e) => ("Ошибка типов", e),
                ParseError::InvalidSyntax(e) => ("Ошибка синтаксиса", e),
                ParseError::ImportError(e) => ("Ошибка импорта", e),
            };
            render_error(&msg, &data);
            return Err((msg.to_string(), data));
        }
    }
    Ok(())
}

fn render_error(msg: &str, error: &ErrorData) {
    let intp = INTERPRETER.read().unwrap();
    let file_name = intp.get_file_path(&error.location.file_id);
    let file_code = intp.source_manager.get_file_content(file_name.as_str());
    let ariadne_span = error.location.as_ariadne(file_code.as_str());

    Report::build(ReportKind::Error, (&file_name, ariadne_span.clone()))
        .with_message(msg)
        .with_label(
            Label::new((&file_name, ariadne_span))
                .with_message(msg)
                .with_color(Color::Red),
        )
        .with_note(&error.message)
        .finish()
        .print(&intp.source_manager)
        .expect("Can't build report message");
}

fn run_repl() {
    println!("Интерактивный режим Гойда. Введите 'выход' для завершения.");
    loop {
        print!("гойда> ");
        io::stdout().flush().unwrap();
        let mut input = String::new();
        if io::stdin().read_line(&mut input).is_ok() {
            let input = input.trim();
            if input == "выход" || input == "exit" {
                break;
            }
            if input.is_empty() {
                continue;
            }
            if let Err(e) = execute_code(input, "repl") {
                eprintln!("Ошибка: {}", e.0);
            }
        }
    }
}
