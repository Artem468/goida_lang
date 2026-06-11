# Goida benchmarks

The suite separates parser/compiler and runtime performance. Each benchmark is
warmed up once, then measured repeatedly. The report uses median (`p50`) for
comparison and `p95` to expose unstable or long-tail runs.

Run the complete suite in release mode:

```bash
cargo run --release -p xtask -- benchmark-suite --iterations 15
```

Save a baseline before changing the runtime:

```bash
cargo run --release -p xtask -- benchmark-suite --iterations 15 --save benchmarks/baseline.tsv
```

Compare the current implementation with that baseline:

```bash
cargo run --release -p xtask -- benchmark-suite --iterations 15 --compare benchmarks/baseline.tsv
```

Positive `change` means execution became slower; negative means faster. Compare
results on the same machine, power profile and build mode. Close background
applications and run the suite more than once before accepting small changes.

The legacy single-file command remains available:

```bash
cargo run --release -p xtask -- benchmark benchmarks/runtime.goida 15
```
