use ariadne::{Color, Label, Report, ReportKind};
use clap::{Parser, Subcommand, ValueEnum};
use std::{
    fs,
    io::{self, Write},
    path::PathBuf,
};

use goida_runtime::diagnostics::{localize_message, DiagnosticLanguage};
use goida_runtime::interpreter::prelude::RuntimeError;
use goida_runtime::parser::prelude::{FormatLanguage, ParseError, Parser as ProgramParser};
use goida_runtime::session::Session;
use goida_runtime::traits::prelude::CoreOperations;
use goida_syntax::ast::prelude::{ErrorData, Span};

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
    #[command(about = "Install all dependencies from goida.toml and update goida.lock")]
    Sync,
    #[command(about = "Synchronize dependencies and build the current package")]
    Build,
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
        #[arg(long, value_enum, default_value_t = FormatLanguageArg::English)]
        language: FormatLanguageArg,
    },
    #[command(about = "Show macro expansion AST preview")]
    ExpandMacros {
        #[arg(help = "Path to a .goida file")]
        file: String,
    },
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum FormatLanguageArg {
    English,
    Russian,
}

impl From<FormatLanguageArg> for FormatLanguage {
    fn from(value: FormatLanguageArg) -> Self {
        match value {
            FormatLanguageArg::English => Self::English,
            FormatLanguageArg::Russian => Self::Russian,
        }
    }
}

fn main() {
    let cli = Cli::parse();
    let mut session = Session::new();
    match &cli.command {
        Some(Commands::Run { file, script_args }) => {
            if let Err((err, _)) = run_file(&mut session, file, script_args) {
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
        Some(Commands::Sync) => exit_on_package_error(package::sync_dependencies()),
        Some(Commands::Build) => exit_on_package_error(package::build_project()),
        Some(Commands::Venv { path }) => exit_on_package_error(package::create_venv(path)),
        Some(Commands::Repl) => run_repl(&mut session),
        Some(Commands::Fmt {
            file,
            write,
            language,
        }) => {
            if let Err(err) = format_file(&session, file, *write, (*language).into()) {
                eprintln!("{err}");
                std::process::exit(1);
            }
        }
        Some(Commands::ExpandMacros { file }) => {
            if let Err(err) = expand_macros_file(&session, file) {
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
        eprintln!("{}", localize_message(&err, DiagnosticLanguage::Russian));
        std::process::exit(1);
    }
}

fn format_file(
    session: &Session,
    file: &str,
    write: bool,
    language: FormatLanguage,
) -> Result<(), String> {
    let source = fs::read_to_string(file).map_err(|err| format!("{}: '{}'", err, file))?;
    let parser = ProgramParser::new(session.interner(), file, PathBuf::from(file));
    let formatted = parser
        .format_source_ast_with_language(&source, language)
        .map_err(|err| format_parse_error(&err))?;
    if write {
        fs::write(file, formatted).map_err(|err| format!("{}: '{}'", err, file))?;
    } else {
        print!("{formatted}");
    }
    Ok(())
}

fn expand_macros_file(session: &Session, file: &str) -> Result<(), String> {
    let source = fs::read_to_string(file).map_err(|err| format!("{}: '{}'", err, file))?;
    let parser = ProgramParser::new(session.interner(), file, PathBuf::from(file));
    match parser.macro_expansion_preview(&source) {
        Ok(preview) => {
            println!("{preview}");
            Ok(())
        }
        Err(err) => Err(format_parse_error_with_language(
            &err,
            DiagnosticLanguage::detect(&source),
        )),
    }
}

fn format_parse_error(err: &ParseError) -> String {
    format_parse_error_with_language(err, DiagnosticLanguage::English)
}

fn format_parse_error_with_language(err: &ParseError, language: DiagnosticLanguage) -> String {
    let (kind, data) = match err {
        ParseError::TypeError(e) => (parse_error_title(err, language), e),
        ParseError::InvalidSyntax(e) => (parse_error_title(err, language), e),
        ParseError::ImportError(e) => (parse_error_title(err, language), e),
    };
    format!("{kind}: {}", localize_message(&data.message, language))
}

fn parse_error_title(error: &ParseError, language: DiagnosticLanguage) -> &'static str {
    match error {
        ParseError::TypeError(_) => language.select("Type error", "Ошибка типов"),
        ParseError::InvalidSyntax(_) => language.select("Syntax error", "Ошибка синтаксиса"),
        ParseError::ImportError(_) => language.select("Import error", "Ошибка импорта"),
    }
}

fn runtime_error_title(error: &RuntimeError, language: DiagnosticLanguage) -> String {
    match error {
        RuntimeError::UndefinedVariable(err) => format!(
            "{}: {}",
            language.select("Undefined variable", "Неопределенная переменная"),
            localize_message(&err.message, language)
        ),
        RuntimeError::UndefinedFunction(err) => format!(
            "{}: {}",
            language.select("Undefined function", "Неопределенная функция"),
            localize_message(&err.message, language)
        ),
        RuntimeError::UndefinedMethod(err) => format!(
            "{}: {}",
            language.select("Undefined method", "Неопределенный метод"),
            localize_message(&err.message, language)
        ),
        RuntimeError::TypeMismatch(err) => format!(
            "{}: {}",
            language.select("Type mismatch", "Несоответствие типов"),
            localize_message(&err.message, language)
        ),
        RuntimeError::Panic(err) => {
            format!(
                "{}: {}",
                language.select("Panic", "Паника"),
                localize_message(&err.message, language)
            )
        }
        RuntimeError::Raised(err, class_name) => {
            format!(
                "{}: {}",
                class_name,
                localize_message(&err.message, language)
            )
        }
        RuntimeError::DivisionByZero(_) => language
            .select("Division by zero", "Деление на ноль")
            .to_string(),
        RuntimeError::InvalidOperation(err) => format!(
            "{}: {}",
            language.select("Invalid operation", "Недопустимая операция"),
            localize_message(&err.message, language)
        ),
        RuntimeError::IOError(err) => format!(
            "{}: {}",
            language.select("I/O error", "Ошибка ввода-вывода"),
            localize_message(&err.message, language)
        ),
        RuntimeError::TypeError(err) => {
            format!(
                "{}: {}",
                language.select("Type error", "Ошибка типа"),
                localize_message(&err.message, language)
            )
        }
        RuntimeError::Return(..) => language
            .select("Unexpected return", "Неожиданный return")
            .to_string(),
        RuntimeError::ImportError(err) => parse_error_title(err, language).to_string(),
    }
}

fn runtime_error_data(error: RuntimeError) -> ErrorData {
    match error {
        RuntimeError::UndefinedVariable(err)
        | RuntimeError::UndefinedFunction(err)
        | RuntimeError::UndefinedMethod(err)
        | RuntimeError::TypeMismatch(err)
        | RuntimeError::DivisionByZero(err)
        | RuntimeError::InvalidOperation(err)
        | RuntimeError::TypeError(err)
        | RuntimeError::IOError(err)
        | RuntimeError::Panic(err)
        | RuntimeError::Raised(err, _)
        | RuntimeError::Return(err, _) => err,
        RuntimeError::ImportError(err) => match err {
            ParseError::TypeError(err)
            | ParseError::InvalidSyntax(err)
            | ParseError::ImportError(err) => err,
        },
    }
}

fn run_file(
    session: &mut Session,
    filename: &str,
    script_args: &[String],
) -> Result<(), (String, ErrorData)> {
    let content = fs::read_to_string(filename).map_err(|e| {
        let msg = format!("{}: '{}'", e, filename);
        (msg.clone(), ErrorData::new(Span::default(), msg))
    })?;
    session.set_diagnostic_language(DiagnosticLanguage::detect(&content));
    session.set_script_args(script_args.to_vec());
    execute_code(session, &content, filename)
}

fn execute_code(
    session: &mut Session,
    code: &str,
    filename: &str,
) -> Result<(), (String, ErrorData)> {
    let path = PathBuf::from(filename);

    let parser = ProgramParser::new(session.interner(), filename, path.clone());
    let _module = parser.module.clone();

    match parser.parse(code) {
        Ok(program) => {
            let interpret_result = session.execute(program);

            interpret_result.map_err(|e| {
                let language = session.diagnostic_language();
                let msg = runtime_error_title(&e, language);
                let error_data = runtime_error_data(e);
                render_error(session, &msg, &error_data);
                (msg, error_data)
            })?;
        }
        Err(err) => {
            session.register_diagnostic_module(_module);
            let language = session.diagnostic_language();
            let msg = parse_error_title(&err, language);
            let data = match err {
                ParseError::TypeError(e)
                | ParseError::InvalidSyntax(e)
                | ParseError::ImportError(e) => e,
            };
            render_error(session, msg, &data);
            return Err((msg.to_string(), data));
        }
    }
    Ok(())
}

fn render_error(session: &Session, msg: &str, error: &ErrorData) {
    let intp = session.runtime();
    let file_name = intp.get_file_path(&error.location.file_id);
    let file_code = intp.source_manager.get_file_content(file_name.as_str());
    let ariadne_span = error.location.as_ariadne(file_code.as_str());
    let display_msg = msg.lines().next().unwrap_or(msg);
    let language = session.diagnostic_language();
    let mut note = localize_message(&error.message, language);

    if !error.stack_trace.is_empty() {
        note.push_str(language.select("\n\nStack trace:", "\n\nСтек вызовов:"));
        for frame in &error.stack_trace {
            let frame_file = intp.get_file_path(&frame.location.file_id);
            let frame_code = intp.source_manager.get_file_content(frame_file.as_str());
            let line = frame_code
                .get(..frame.location.start.min(frame_code.len() as u32) as usize)
                .map(|prefix| prefix.lines().count())
                .unwrap_or(0)
                + 1;
            let at = language.select("at", "в");
            note.push_str(&format!(
                "\n  {at} {} ({}:{})",
                frame.name, frame_file, line
            ));
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

fn run_repl(session: &mut Session) {
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
            if let Err(e) = execute_code(session, input, "repl") {
                eprintln!("Ошибка: {}", e.0.lines().next().unwrap_or(&e.0));
            }
        }
    }
}
