// =============================================================================
// VIL REST Handlers — Model Serving
// =============================================================================

use std::sync::Arc;

use serde::{Deserialize, Serialize};
use vil_server::prelude::*;

use crate::serving::ModelServer;
use crate::metrics::VariantMetrics;

// ── Request / Response types ────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct InferRequest {
    pub messages: Vec<InferMessageInput>,
}

#[derive(Debug, Deserialize)]
pub struct InferMessageInput {
    pub role: String,
    pub content: String,
}

#[derive(Debug, Serialize)]
pub struct InferResponse {
    pub content: String,
    pub variant_name: String,
    pub version: u32,
    pub latency_ms: u64,
}

#[derive(Debug, Serialize)]
pub struct ModelsResponse {
    pub variant_count: usize,
    pub variants: Vec<(String, VariantMetrics)>,
}

#[derive(Debug, Serialize)]
pub struct ServingStatsResponse {
    pub variant_count: usize,
    pub metrics: Vec<(String, VariantMetrics)>,
}

// ── Handlers ────────────────────────────────────────────────────────────────

/// POST /api/serving/infer — run inference through weighted variant selection.
pub async fn handle_infer(
    ctx: ServiceCtx,
    body: ShmSlice,
) -> HandlerResult<VilResponse<InferResponse>> {
    let server = ctx.state::<Arc<ModelServer>>().expect("ModelServer");
    let req: InferRequest = body.json().expect("invalid JSON");
    let messages: Vec<vil_llm::ChatMessage> = req
        .messages
        .iter()
        .map(|m| vil_llm::ChatMessage::user(&m.content))
        .collect();

    match server.serve(&messages).await {
        Ok(result) => {
            let resp = InferResponse {
                content: result.content,
                variant_name: result.variant_name,
                version: result.version,
                latency_ms: result.latency_ms,
            };
            Ok(VilResponse::ok(resp))
        }
        Err(e) => Err(VilError::internal(e.to_string())),
    }
}

/// GET /api/serving/models — list active model variants.
pub async fn handle_models(
    ctx: ServiceCtx,
) -> HandlerResult<VilResponse<ModelsResponse>> {
    let server = ctx.state::<Arc<ModelServer>>().expect("ModelServer");
    let variants = server.get_metrics();
    let resp = ModelsResponse {
        variant_count: server.variant_count(),
        variants,
    };
    Ok(VilResponse::ok(resp))
}

/// GET /api/serving/stats — return serving statistics.
pub async fn handle_stats(
    ctx: ServiceCtx,
) -> HandlerResult<VilResponse<ServingStatsResponse>> {
    let server = ctx.state::<Arc<ModelServer>>().expect("ModelServer");
    let metrics = server.get_metrics();
    let resp = ServingStatsResponse {
        variant_count: server.variant_count(),
        metrics,
    };
    Ok(VilResponse::ok(resp))
}
