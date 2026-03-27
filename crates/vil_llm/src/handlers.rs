//! VIL pattern HTTP handlers for the LLM plugin.
//!
//! All handlers follow VIL conventions:
//! - Extract shared state via `Extension<T>`
//! - Return `HandlerResult<VilResponse<T>>` or `VilResponse<T>`
//! - Use `VilError` for structured error responses

use vil_server::prelude::*;
use serde::{Deserialize, Serialize};

use crate::extractors::{Embedder, Llm};
use crate::message::ChatMessage;

/// Combined service state for the LLM plugin.
pub struct LlmServiceState {
    pub llm: Llm,
    pub embedder: Option<Embedder>,
    pub models: ModelsResponseBody,
}

// ── Request / Response types ────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct ChatRequest {
    pub messages: Vec<ChatMessage>,
    #[serde(default)]
    pub tools: Option<Vec<serde_json::Value>>,
}

#[derive(Debug, Serialize)]
pub struct ChatResponseBody {
    pub content: String,
    pub model: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<crate::message::ToolCall>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub usage: Option<crate::message::Usage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub finish_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct EmbedRequest {
    pub texts: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct EmbedResponseBody {
    pub embeddings: Vec<Vec<f32>>,
    pub dimension: usize,
    pub count: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct ModelsResponseBody {
    pub models: Vec<String>,
}

// ── Handlers ────────────────────────────────────────────────────────

/// POST /chat — Chat completion via configured LLM provider.
pub async fn chat_handler(
    ctx: ServiceCtx,
    body: ShmSlice,
) -> HandlerResult<VilResponse<ChatResponseBody>> {
    let state = ctx.state::<LlmServiceState>()?;
    let llm = &state.llm;
    let req: ChatRequest = body.json().map_err(|e| VilError::bad_request(e.to_string()))?;
    if req.messages.is_empty() {
        return Err(VilError::bad_request("messages must not be empty"));
    }

    let resp = match req.tools {
        Some(ref tools) if !tools.is_empty() => {
            llm.chat_with_tools(&req.messages, tools).await
        }
        _ => llm.chat(&req.messages).await,
    };

    let resp = resp.map_err(|e| VilError::internal(e.to_string()))?;

    Ok(VilResponse::ok(ChatResponseBody {
        content: resp.content,
        model: resp.model,
        tool_calls: resp.tool_calls,
        usage: resp.usage,
        finish_reason: resp.finish_reason,
    }))
}

/// POST /embed — Generate embeddings for input texts.
pub async fn embed_handler(
    ctx: ServiceCtx,
    body: ShmSlice,
) -> HandlerResult<VilResponse<EmbedResponseBody>> {
    let state = ctx.state::<LlmServiceState>()?;
    let embedder = state.embedder.as_ref()
        .ok_or_else(|| VilError::internal("no embedder configured"))?;
    let req: EmbedRequest = body.json().map_err(|e| VilError::bad_request(e.to_string()))?;
    if req.texts.is_empty() {
        return Err(VilError::bad_request("texts must not be empty"));
    }

    let embeddings = embedder
        .embed(&req.texts)
        .await
        .map_err(|e| VilError::internal(e.to_string()))?;

    let dim = embedder.dimension();
    let count = embeddings.len();

    Ok(VilResponse::ok(EmbedResponseBody {
        embeddings,
        dimension: dim,
        count,
    }))
}

/// GET /models — List available LLM models.
pub async fn models_handler(
    ctx: ServiceCtx,
) -> VilResponse<ModelsResponseBody> {
    let state = ctx.state::<LlmServiceState>().expect("LlmServiceState");
    VilResponse::ok(state.models.clone())
}
