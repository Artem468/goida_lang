use goida_runtime::builtins::registry::BUILTINS;
use goida_runtime::parser::prelude::Parser;
use goida_runtime::session::Session;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

const BUILTIN_DOCS_PATH: &str = "docs/builtins.md";
const DEFAULT_BENCHMARK_PATH: &str = "benchmarks/runtime.goida";

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    match args.next().as_deref() {
        Some("builtin-docs") => generate_markdown_docs(args.any(|arg| arg == "--check")),
        Some("benchmark") => {
            let path = args
                .next()
                .map(PathBuf::from)
                .unwrap_or_else(|| PathBuf::from(DEFAULT_BENCHMARK_PATH));
            let iterations = args
                .next()
                .map(|value| value.parse())
                .transpose()?
                .unwrap_or(10);
            benchmark(&path, iterations)
        }
        _ => Err(
            "usage: cargo run -p xtask -- <builtin-docs [--check] | benchmark [path] [iterations]>"
                .into(),
        ),
    }
}

fn benchmark(path: &Path, iterations: usize) -> Result<(), Box<dyn std::error::Error>> {
    if iterations == 0 {
        return Err("benchmark iterations must be greater than zero".into());
    }
    let source = std::fs::read_to_string(path)?;
    let mut parse_total = Duration::ZERO;
    let mut execute_total = Duration::ZERO;
    let mut module_registers = 0;
    let mut max_body_registers = 0;

    for _ in 0..iterations {
        let mut session = Session::new();
        let started = Instant::now();
        let module = Parser::new(
            session.interner(),
            &path.to_string_lossy(),
            path.to_path_buf(),
        )
        .parse(&source)
        .map_err(|error| format!("benchmark parse failed: {error:?}"))?;
        parse_total += started.elapsed();
        module_registers = module.bytecode.module.register_count;
        max_body_registers = module
            .bytecode
            .bodies
            .values()
            .chain(module.bytecode.expressions.values())
            .map(|chunk| chunk.register_count)
            .max()
            .unwrap_or_default();

        let started = Instant::now();
        session
            .execute(module)
            .map_err(|error| format!("benchmark execution failed: {error:?}"))?;
        execute_total += started.elapsed();
    }

    println!(
        "iterations={iterations} parse_avg_ms={:.3} execute_avg_ms={:.3} \
         module_registers={module_registers} max_body_registers={max_body_registers}",
        parse_total.as_secs_f64() * 1000.0 / iterations as f64,
        execute_total.as_secs_f64() * 1000.0 / iterations as f64,
    );
    Ok(())
}

fn generate_markdown_docs(check: bool) -> Result<(), Box<dyn std::error::Error>> {
    let generated = BUILTINS.generate_markdown_docs();
    let output = Path::new(BUILTIN_DOCS_PATH);

    if check {
        let current = std::fs::read_to_string(output).unwrap_or_default();
        if current != generated {
            return Err(format!(
                "{BUILTIN_DOCS_PATH} is outdated; run `cargo run -p xtask -- builtin-docs`"
            )
            .into());
        }
        return Ok(());
    }

    if let Some(parent) = output.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(output, generated)?;
    println!("Generated {BUILTIN_DOCS_PATH}");
    Ok(())
}
