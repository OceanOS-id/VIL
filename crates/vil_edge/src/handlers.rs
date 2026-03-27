//! HTTP handlers for the edge plugin — wired to real EdgeRuntime state.

use vil_server::prelude::*;
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::runtime::EdgeRuntime;

// ── Response types ──────────────────────────────────────────────────

#[derive(Debug, Serialize)]
pub struct EdgeModelSummary {
    pub name: String,
    pub format: String,
    pub size_mb: u64,
    pub quantization: String,
}

#[derive(Debug, Serialize)]
pub struct EdgeStatsBody {
    pub model_count: usize,
    pub models: Vec<EdgeModelSummary>,
    pub max_memory_mb: u64,
    pub max_model_size_mb: u64,
    pub target_arch: String,
}

// ── Handlers ────────────────────────────────────────────────────────

/// GET /stats — return real edge runtime model count and config.
pub async fn stats_handler(
    ctx: ServiceCtx,
) -> HandlerResult<VilResponse<EdgeStatsBody>> {
    let runtime = ctx.state::<Arc<Mutex<EdgeRuntime>>>()?;
    let rt = runtime.lock().await;

    let models: Vec<EdgeModelSummary> = rt
        .models
        .iter()
        .map(|m| EdgeModelSummary {
            name: m.name.clone(),
            format: m.format.to_string(),
            size_mb: m.size_mb,
            quantization: format!("{:?}", m.quantization),
        })
        .collect();

    Ok(VilResponse::ok(EdgeStatsBody {
        model_count: models.len(),
        models,
        max_memory_mb: rt.config.max_memory_mb,
        max_model_size_mb: rt.config.max_model_size_mb,
        target_arch: rt.config.target_arch.to_string(),
    }))
}
