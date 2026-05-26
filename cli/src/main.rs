use ariadne::{Color, Label, Report, ReportKind};
use clap::{Parser, Subcommand};
use std::{
    fs,
    io::{self, Write},
    path::PathBuf,
};

use goida_core::ast::prelude::{ErrorData, Span};
use goida_core::formatter::format_source;
use goida_core::interpreter::prelude::RuntimeError;
use goida_core::parser::prelude::{ParseError, Parser as ProgramParser};
use goida_core::traits::prelude::CoreOperations;
use goida_core::INTERPRETER;

mod package;

#[derive(Parser)]
#[command(
    name = "goida",
    about = "Интерпретатор языка программирования Гойда",
    disable_help_subcommand = true
)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    #[command(about = "Запустить .goida файл")]
    Run {
        #[arg(help = "Путь к исходному .goida файлу")]
        file: String,
        #[arg(
            trailing_var_arg = true,
            allow_hyphen_values = true,
            help = "Дополнительные аргументы скрипта"
        )]
        script_args: Vec<String>,
    },
    #[command(about = "Создать новый проект")]
    New {
        #[arg(help = "Имя каталога проекта и имя пакета")]
        name: String,
        #[arg(long, default_value = "", help = "Описание проекта")]
        description: String,
        #[arg(long, default_value = "0.1.0", help = "Версия проекта")]
        version: String,
    },
    #[command(about = "Добавить зависимость из git или локального каталога")]
    Add {
        #[arg(help = "Локальное имя зависимости")]
        name: String,
        #[arg(long, help = "Git URL или путь к git-репозиторию")]
        git: Option<String>,
        #[arg(long, help = "Путь к локальному каталогу зависимости")]
        path: Option<String>,
        #[arg(long, help = "Commit или git-ссылка, только для --git")]
        rev: Option<String>,
        #[arg(long, help = "Ветка, только для --git")]
        branch: Option<String>,
        #[arg(long, help = "Тег, только для --git")]
        tag: Option<String>,
    },
    #[command(about = "Удалить зависимость")]
    Remove {
        #[arg(help = "Имя зависимости")]
        name: String,
    },
    #[command(about = "Создать виртуальное окружение Гойда")]
    Venv {
        #[arg(default_value = ".goida", help = "Путь к каталогу окружения")]
        path: String,
    },
    #[command(about = "Запустить интерактивный режим")]
    Repl,
    #[command(about = "Format a .goida file")]
    Fmt {
        #[arg(help = "Path to a .goida file")]
        file: String,
        #[arg(long, help = "Rewrite the file in place")]
        write: bool,
    },
    #[command(about = "Show macro expansion AST preview")]
    ExpandMacros {
        #[arg(help = "Path to a .goida file")]
        file: String,
    },
}

fn main() {
    let cli = Cli::parse();
    match &cli.command {
        Some(Commands::Run { file, .. }) => {
            if let Err((err, _)) = run_file(file) {
                println!("{}", err.lines().next().unwrap_or(&err));
                std::process::exit(1);
            }
        }
        Some(Commands::New {
            name,
            description,
            version,
        }) => exit_on_package_error(package::new_project(name, description, version)),
        Some(Commands::Add {
            name,
            git,
            path,
            rev,
            branch,
            tag,
        }) => exit_on_package_error(package::add_dependency(
            name,
            git.clone(),
            path.clone(),
            rev.clone(),
            branch.clone(),
            tag.clone(),
        )),
        Some(Commands::Remove { name }) => exit_on_package_error(package::remove_dependency(name)),
        Some(Commands::Venv { path }) => exit_on_package_error(package::create_venv(path)),
        Some(Commands::Repl) => run_repl(),
        Some(Commands::Fmt { file, write }) => {
            if let Err(err) = format_file(file, *write) {
                eprintln!("{err}");
                std::process::exit(1);
            }
        }
        Some(Commands::ExpandMacros { file }) => {
            if let Err(err) = expand_macros_file(file) {
                eprintln!("{err}");
                std::process::exit(1);
            }
        }
        None => {
            println!("Добро пожаловать в Гойда! Используйте --help для справки.");
        }
    }
}

fn exit_on_package_error(result: Result<(), String>) {
    if let Err(err) = result {
        eprintln!("{err}");
        std::process::exit(1);
    }
}

fn format_file(file: &str, write: bool) -> Result<(), String> {
    let source = fs::read_to_string(file).map_err(|err| format!("{}: '{}'", err, file))?;
    let formatted = format_source(&source);
    if write {
        fs::write(file, formatted).map_err(|err| format!("{}: '{}'", err, file))?;
    } else {
        print!("{formatted}");
    }
    Ok(())
}

fn expand_macros_file(file: &str) -> Result<(), String> {
    let source = fs::read_to_string(file).map_err(|err| format!("{}: '{}'", err, file))?;
    let parser = ProgramParser::new(
        INTERPRETER.read().unwrap().interner.clone(),
        file,
        PathBuf::from(file),
    );
    match parser.macro_expansion_preview(&source) {
        Ok(preview) => {
            println!("{preview}");
            Ok(())
        }
        Err(err) => {
            let (kind, data) = match err {
                ParseError::TypeError(e) => ("Ошибка типов", e),
                ParseError::InvalidSyntax(e) => ("Ошибка синтаксиса", e),
                ParseError::ImportError(e) => ("Ошибка импорта", e),
            };
            Err(format!("{kind}: {}", data.message))
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
                    RuntimeError::Raised(err, class_name) => {
                        (format!("{}: {}", class_name, err.message), err)
                    }
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
            render_error(msg, &data);
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
    let display_msg = msg.lines().next().unwrap_or(msg);
    let mut note = error.message.clone();

    if !error.stack_trace.is_empty() {
        note.push_str("\n\nСтек вызовов:");
        for frame in &error.stack_trace {
            let frame_file = intp.get_file_path(&frame.location.file_id);
            let frame_code = intp.source_manager.get_file_content(frame_file.as_str());
            let line = frame_code
                .get(..frame.location.start.min(frame_code.len() as u32) as usize)
                .map(|prefix| prefix.lines().count())
                .unwrap_or(0)
                + 1;
            note.push_str(&format!("\n  в {} ({}:{})", frame.name, frame_file, line));
        }
    }

    Report::build(ReportKind::Error, (&file_name, ariadne_span.clone()))
        .with_message(display_msg)
        .with_label(
            Label::new((&file_name, ariadne_span))
                .with_message(display_msg)
                .with_color(Color::Red),
        )
        .with_note(note)
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
                eprintln!("Ошибка: {}", e.0.lines().next().unwrap_or(&e.0));
            }
        }
    }
}
