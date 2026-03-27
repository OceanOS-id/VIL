// ── vil_bench_llm ── N04: LLM Benchmark Suite ─────────────────────
//!
//! Pluggable LLM benchmark framework with built-in math, logic, and
//! factual Q&A benchmarks.  Implement `Benchmark` to add custom suites.

pub mod benchmark;
pub mod built_in;
pub mod report;
pub mod suite;

pub use benchmark::{BenchCase, Benchmark};
pub use built_in::{FactBench, LogicBench, MathBench};
pub use report::{BenchReport, BenchResult};
pub use suite::BenchSuite;

// VIL integration layer
pub mod vil_semantic;
pub mod pipeline_sse;
pub mod handlers;
pub mod plugin;

pub use plugin::BenchPlugin;
pub use vil_semantic::{BenchEvent, BenchFault, BenchState};
