//! HTTP handlers for the data prep plugin — wired to real DataPipeline state.

use std::sync::Arc;
use vil_server::prelude::*;

use crate::pipeline::{DataPipeline, PipelineStep};

// ── Response types ──────────────────────────────────────────────────

#[derive(Debug, Serialize)]
pub struct DataPrepStatsBody {
    pub step_count: usize,
    pub steps: Vec<String>,
    pub formats: Vec<String>,
    pub version: String,
}

// ── Handlers ────────────────────────────────────────────────────────

/// GET /stats — return real pipeline step configuration.
pub async fn stats_handler(ctx: ServiceCtx) -> HandlerResult<VilResponse<DataPrepStatsBody>> {
    let pipeline = ctx.state::<Arc<DataPipeline>>().expect("DataPipeline");
    let steps: Vec<String> = pipeline
        .steps
        .iter()
        .map(|s| match s {
            PipelineStep::Dedup => "exact_dedup".into(),
            PipelineStep::FuzzyDedup { threshold } => format!("fuzzy_dedup(threshold={threshold})"),
            PipelineStep::Filter(qf) => format!(
                "quality_filter(min_len={}, max_len={})",
                qf.min_length, qf.max_length
            ),
            PipelineStep::Format(fmt) => format!("format({fmt:?})"),
            PipelineStep::Custom(name) => format!("custom({name})"),
        })
        .collect();

    Ok(VilResponse::ok(DataPrepStatsBody {
        step_count: steps.len(),
        steps,
        formats: vec![
            "jsonl".into(),
            "alpaca".into(),
            "sharegpt".into(),
            "chatml".into(),
        ],
        version: env!("CARGO_PKG_VERSION").into(),
    }))
}
