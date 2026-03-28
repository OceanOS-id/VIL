//! HTTP handlers for the cost tracker plugin — wired to real CostTracker state.

use std::sync::Arc;
use vil_server::prelude::*;

use crate::tracker::{CostTracker, ModelCostEntry};

// ── Response types ──────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize)]
pub struct CostStatsBody {
    pub total_cost_usd: f64,
    pub total_requests: u64,
    pub model_count: usize,
    pub models: Vec<ModelCostEntry>,
}

// ── Handlers ────────────────────────────────────────────────────────

/// GET /stats — return real cost data from the tracker.
pub async fn stats_handler(ctx: ServiceCtx) -> HandlerResult<VilResponse<CostStatsBody>> {
    let tracker = ctx.state::<Arc<CostTracker>>().expect("CostTracker");
    let report = tracker.cost_report();

    Ok(VilResponse::ok(CostStatsBody {
        total_cost_usd: report.total_cost_usd,
        total_requests: report.total_requests,
        model_count: report.models.len(),
        models: report.models,
    }))
}
