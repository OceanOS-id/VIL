//! VIL pattern HTTP handlers for the LLM proxy plugin.
use crate::metrics::ProxyMetrics;
use crate::proxy::LlmProxy;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use vil_server::prelude::*;

/// Combined service state for the LLM proxy (proxy + metrics).
pub struct LlmProxyState {
    pub proxy: Arc<LlmProxy>,
    pub metrics: Arc<ProxyMetrics>,
}

#[derive(Debug, Deserialize)]
pub struct ProxyChatRequest {
    pub messages: Vec<vil_llm::ChatMessage>,
    #[serde(default = "default_api_key")]
    pub api_key: String,
}

fn default_api_key() -> String {
    "default".into()
}

#[derive(Debug, Serialize)]
pub struct ProxyChatResponse {
    pub content: String,
    pub model: String,
}

#[derive(Debug, Serialize)]
pub struct ProxyStatsResponse {
    pub total_requests: u64,
    pub cache_hits: u64,
    pub cache_misses: u64,
}

pub async fn proxy_chat_handler(
    ctx: ServiceCtx,
    body: ShmSlice,
) -> HandlerResult<VilResponse<ProxyChatResponse>> {
    let state = ctx.state::<LlmProxyState>()?;
    let proxy = &state.proxy;
    let req: ProxyChatRequest = body
        .json()
        .map_err(|e| VilError::bad_request(e.to_string()))?;
    if req.messages.is_empty() {
        return Err(VilError::bad_request("messages must not be empty"));
    }
    let result = proxy
        .chat(&req.api_key, &req.messages)
        .await
        .map_err(|e| VilError::internal(e.to_string()))?;
    Ok(VilResponse::ok(ProxyChatResponse {
        content: result.content,
        model: result.model,
    }))
}

pub async fn proxy_stats_handler(ctx: ServiceCtx) -> VilResponse<ProxyStatsResponse> {
    let state = ctx.state::<LlmProxyState>().expect("LlmProxyState");
    let snap = state.metrics.snapshot();
    VilResponse::ok(ProxyStatsResponse {
        total_requests: snap.total_requests,
        cache_hits: snap.cache_hits,
        cache_misses: snap.cache_misses,
    })
}
