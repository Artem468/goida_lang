use goida_core::builtins::registry::BUILTINS;
use std::path::Path;

const BUILTIN_DOCS_PATH: &str = "docs/builtins.md";

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    match args.next().as_deref() {
        Some("builtin-docs") => generate_markdown_docs(args.any(|arg| arg == "--check")),
        _ => Err("usage: cargo run -p xtask -- builtin-docs [--check]".into()),
    }
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
