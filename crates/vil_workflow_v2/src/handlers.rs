//! VIL pattern HTTP handlers for the workflow plugin.

use vil_server::prelude::*;

use std::sync::Arc;

use crate::scheduler::WorkflowScheduler;

// ── Response types ──────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize)]
pub struct WorkflowStatsBody {
    pub scheduler: String,
    pub dag_resolver: String,
    pub version: String,
}

// ── Handlers ────────────────────────────────────────────────────────

/// GET /stats — Workflow service stats.
pub async fn stats_handler(ctx: ServiceCtx) -> VilResponse<WorkflowStatsBody> {
    VilResponse::ok(WorkflowStatsBody {
        scheduler: "WorkflowScheduler".into(),
        dag_resolver: "resolve_layers".into(),
        version: env!("CARGO_PKG_VERSION").into(),
    })
}
