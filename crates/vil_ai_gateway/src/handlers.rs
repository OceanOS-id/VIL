// =============================================================================
// VIL REST Handlers — AI Gateway
// =============================================================================

use std::sync::Arc;

use serde::{Deserialize, Serialize};
use vil_server::prelude::*;

use crate::gateway::AiGateway;
use crate::metrics::MetricsSnapshot;
use crate::health::ModelHealth;

// ── Request / Response types ────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct ChatRequest {
    pub messages: Vec<ChatMessageInput>,
}

#[derive(Debug, Deserialize)]
pub struct ChatMessageInput {
    pub role: String,
    pub content: String,
}

#[derive(Debug, Serialize)]
pub struct ChatResponse {
    pub content: String,
    pub model_used: String,
    pub latency_ms: u64,
    pub cost_usd: f64,
    pub attempts: u32,
}

#[derive(Debug, Serialize)]
pub struct GatewayStatsResponse {
    pub metrics: MetricsSnapshot,
}

#[derive(Debug, Serialize)]
pub struct GatewayHealthResponse {
    pub models: Vec<ModelHealth>,
}

// ── Handlers ────────────────────────────────────────────────────────────────

/// POST /api/gateway/chat — route a chat request through the gateway.
pub async fn handle_chat(
    ctx: ServiceCtx,
    body: ShmSlice,
) -> HandlerResult<VilResponse<ChatResponse>> {
    let gateway = ctx.state::<Arc<AiGateway>>().expect("AiGateway");
    let req: ChatRequest = body.json().expect("invalid JSON");
    let messages: Vec<vil_llm::ChatMessage> = req
        .messages
        .iter()
        .map(|m| vil_llm::ChatMessage::user(&m.content))
        .collect();

    match gateway.chat(&messages).await {
        Ok(resp) => {
            let out = ChatResponse {
                content: resp.content,
                model_used: resp.model_used,
                latency_ms: resp.latency_ms,
                cost_usd: resp.cost_usd,
                attempts: resp.attempts,
            };
            Ok(VilResponse::ok(out))
        }
        Err(e) => Err(VilError::internal(e.to_string())),
    }
}

/// GET /api/gateway/stats — return gateway metrics.
pub async fn handle_stats(
    ctx: ServiceCtx,
) -> HandlerResult<VilResponse<GatewayStatsResponse>> {
    let gateway = ctx.state::<Arc<AiGateway>>().expect("AiGateway");
    let metrics = gateway.metrics();
    let resp = GatewayStatsResponse { metrics };
    Ok(VilResponse::ok(resp))
}

/// GET /api/gateway/health — return per-model health status.
pub async fn handle_health(
    ctx: ServiceCtx,
) -> HandlerResult<VilResponse<GatewayHealthResponse>> {
    let gateway = ctx.state::<Arc<AiGateway>>().expect("AiGateway");
    let models = gateway.health();
    let resp = GatewayHealthResponse { models };
    Ok(VilResponse::ok(resp))
}
