//! HTTP handlers for the benchmark plugin — wired to real BenchSuite state.

use std::sync::Arc;
use vil_server::prelude::*;

use crate::suite::BenchSuite;

// ── Response types ──────────────────────────────────────────────────

#[derive(Debug, Serialize)]
pub struct BenchStatsBody {
    pub benchmark_count: usize,
    pub benchmarks: Vec<String>,
    pub version: String,
}

// ── Handlers ────────────────────────────────────────────────────────

/// GET /stats — return available benchmarks from the suite.
pub async fn stats_handler(ctx: ServiceCtx) -> HandlerResult<VilResponse<BenchStatsBody>> {
    let suite = ctx.state::<Arc<BenchSuite>>().expect("BenchSuite");
    let benchmarks: Vec<String> = suite
        .benchmarks
        .iter()
        .map(|b| b.name().to_string())
        .collect();
    let benchmark_count = benchmarks.len();

    Ok(VilResponse::ok(BenchStatsBody {
        benchmark_count,
        benchmarks,
        version: env!("CARGO_PKG_VERSION").into(),
    }))
}
