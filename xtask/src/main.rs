use goida_runtime::builtins::registry::BUILTINS;
use goida_runtime::parser::prelude::Parser;
use goida_runtime::session::Session;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

const BUILTIN_DOCS_PATH: &str = "docs/builtins.md";
const DEFAULT_BENCHMARK_PATH: &str = "benchmarks/runtime.goida";
const BENCHMARKS_DIR: &str = "benchmarks/suite";
const DEFAULT_BENCHMARK_ITERATIONS: usize = 10;

#[derive(Clone, Debug)]
struct BenchmarkResult {
    name: String,
    parse_median: Duration,
    parse_p95: Duration,
    execute_median: Duration,
    execute_p95: Duration,
    module_registers: u32,
    max_body_registers: u32,
}

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
                .unwrap_or(DEFAULT_BENCHMARK_ITERATIONS);
            benchmark(&path, iterations)
        }
        Some("benchmark-suite") => benchmark_suite(args.collect()),
        _ => Err(
            "usage: cargo run -p xtask -- <builtin-docs [--check] | benchmark [path] [iterations] | benchmark-suite [--iterations N] [--save PATH] [--compare PATH]>".into(),
        ),
    }
}

fn benchmark(path: &Path, iterations: usize) -> Result<(), Box<dyn std::error::Error>> {
    let result = run_benchmark(path, iterations)?;
    print_results(std::slice::from_ref(&result), None);
    Ok(())
}

fn benchmark_suite(args: Vec<String>) -> Result<(), Box<dyn std::error::Error>> {
    let mut iterations = DEFAULT_BENCHMARK_ITERATIONS;
    let mut save = None;
    let mut compare = None;
    let mut index = 0;
    while index < args.len() {
        let option = &args[index];
        let value = args
            .get(index + 1)
            .ok_or_else(|| format!("{option} requires a value"))?;
        match option.as_str() {
            "--iterations" => iterations = value.parse()?,
            "--save" => save = Some(PathBuf::from(value)),
            "--compare" => compare = Some(PathBuf::from(value)),
            _ => return Err(format!("unknown benchmark-suite option: {option}").into()),
        }
        index += 2;
    }

    let mut paths = std::fs::read_dir(BENCHMARKS_DIR)?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| {
            path.extension()
                .is_some_and(|extension| extension == "goida")
        })
        .collect::<Vec<_>>();
    paths.sort();
    if paths.is_empty() {
        return Err(format!("no .goida benchmarks found in {BENCHMARKS_DIR}").into());
    }

    let mut results = Vec::with_capacity(paths.len());
    for path in paths {
        results.push(run_benchmark(&path, iterations)?);
    }

    let baseline = compare
        .as_deref()
        .map(read_results)
        .transpose()?
        .unwrap_or_default();
    print_results(&results, Some(&baseline));
    if let Some(path) = save {
        write_results(&path, &results)?;
        println!("\nsaved baseline: {}", path.display());
    }
    Ok(())
}

fn run_benchmark(
    path: &Path,
    iterations: usize,
) -> Result<BenchmarkResult, Box<dyn std::error::Error>> {
    if iterations == 0 {
        return Err("benchmark iterations must be greater than zero".into());
    }
    let source = std::fs::read_to_string(path)?;
    let mut parse_samples = Vec::with_capacity(iterations);
    let mut execute_samples = Vec::with_capacity(iterations);
    let mut module_registers = 0;
    let mut max_body_registers = 0;

    // The first run pays one-time allocator and OS costs and is intentionally excluded.
    for iteration in 0..=iterations {
        let mut session = Session::new();
        let started = Instant::now();
        let module = Parser::new(
            session.interner(),
            &path.to_string_lossy(),
            path.to_path_buf(),
        )
        .parse(&source)
        .map_err(|error| format!("benchmark parse failed for {}: {error:?}", path.display()))?;
        let parse_elapsed = started.elapsed();
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
        session.execute(module).map_err(|error| {
            format!(
                "benchmark execution failed for {}: {error:?}",
                path.display()
            )
        })?;
        let execute_elapsed = started.elapsed();
        if iteration > 0 {
            parse_samples.push(parse_elapsed);
            execute_samples.push(execute_elapsed);
        }
    }

    parse_samples.sort();
    execute_samples.sort();
    Ok(BenchmarkResult {
        name: path
            .file_stem()
            .and_then(|name| name.to_str())
            .unwrap_or("benchmark")
            .to_string(),
        parse_median: percentile(&parse_samples, 0.50),
        parse_p95: percentile(&parse_samples, 0.95),
        execute_median: percentile(&execute_samples, 0.50),
        execute_p95: percentile(&execute_samples, 0.95),
        module_registers,
        max_body_registers,
    })
}

fn percentile(samples: &[Duration], percentile: f64) -> Duration {
    let index = ((samples.len() - 1) as f64 * percentile).ceil() as usize;
    samples[index]
}

fn print_results(results: &[BenchmarkResult], baseline: Option<&[BenchmarkResult]>) {
    println!(
        "{:<20} {:>10} {:>10} {:>10} {:>10} {:>9} {:>9} {:>9}",
        "benchmark",
        "parse p50",
        "parse p95",
        "exec p50",
        "exec p95",
        "change",
        "module r",
        "body r"
    );
    for result in results {
        let change = baseline
            .and_then(|baseline| baseline.iter().find(|item| item.name == result.name))
            .map_or_else(
                || "-".to_string(),
                |old| {
                    let ratio =
                        result.execute_median.as_secs_f64() / old.execute_median.as_secs_f64();
                    format!("{:+.1}%", (ratio - 1.0) * 100.0)
                },
            );
        println!(
            "{:<20} {:>9.3}ms {:>9.3}ms {:>9.3}ms {:>9.3}ms {:>9} {:>9} {:>9}",
            result.name,
            duration_ms(result.parse_median),
            duration_ms(result.parse_p95),
            duration_ms(result.execute_median),
            duration_ms(result.execute_p95),
            change,
            result.module_registers,
            result.max_body_registers,
        );
    }
}

fn write_results(
    path: &Path,
    results: &[BenchmarkResult],
) -> Result<(), Box<dyn std::error::Error>> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let mut output =
        String::from("name\tparse_p50_ns\tparse_p95_ns\texec_p50_ns\texec_p95_ns\tmodule_registers\tmax_body_registers\n");
    for result in results {
        output.push_str(&format!(
            "{}\t{}\t{}\t{}\t{}\t{}\t{}\n",
            result.name,
            result.parse_median.as_nanos(),
            result.parse_p95.as_nanos(),
            result.execute_median.as_nanos(),
            result.execute_p95.as_nanos(),
            result.module_registers,
            result.max_body_registers,
        ));
    }
    std::fs::write(path, output)?;
    Ok(())
}

fn read_results(path: &Path) -> Result<Vec<BenchmarkResult>, Box<dyn std::error::Error>> {
    let source = std::fs::read_to_string(path)?;
    source
        .lines()
        .skip(1)
        .filter(|line| !line.trim().is_empty())
        .map(|line| {
            let fields = line.split('\t').collect::<Vec<_>>();
            if fields.len() != 7 {
                return Err(format!("invalid benchmark baseline row: {line}").into());
            }
            Ok(BenchmarkResult {
                name: fields[0].to_string(),
                parse_median: Duration::from_nanos(fields[1].parse()?),
                parse_p95: Duration::from_nanos(fields[2].parse()?),
                execute_median: Duration::from_nanos(fields[3].parse()?),
                execute_p95: Duration::from_nanos(fields[4].parse()?),
                module_registers: fields[5].parse()?,
                max_body_registers: fields[6].parse()?,
            })
        })
        .collect()
}

fn duration_ms(duration: Duration) -> f64 {
    duration.as_secs_f64() * 1000.0
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
