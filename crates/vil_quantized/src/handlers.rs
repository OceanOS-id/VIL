//! VIL pattern HTTP handlers for the Quantized Runtime plugin.
//!
//! All handlers follow VIL conventions:
//! - Extract shared state via `ServiceCtx`
//! - Return `HandlerResult<VilResponse<T>>` or `VilResponse<T>`
//! - Use `VilError` for structured error responses

use serde::{Deserialize, Serialize};
use vil_server::prelude::*;

use crate::runtime::QuantizedRuntime;
use crate::semantic::{QuantizeEvent, QuantizedState};

use std::sync::{Arc, Mutex};

/// Combined service state for Quantized Runtime handlers.
pub struct QuantizedServiceState {
    pub runtime: Arc<Mutex<QuantizedRuntime>>,
    pub state: Arc<Mutex<QuantizedState>>,
}

// ── Request / Response types ────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct InferRequest {
    pub prompt: String,
    #[serde(default = "default_max_tokens")]
    pub max_tokens: usize,
}

fn default_max_tokens() -> usize {
    256
}

#[derive(Debug, Serialize)]
pub struct InferResponseBody {
    pub text: String,
    pub model_path: String,
    pub format: String,
    pub memory_mb: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct QuantizedStatsBody {
    pub total_inferences: u64,
    pub total_errors: u64,
    pub total_tokens_generated: u64,
    pub model_loaded: bool,
    pub model_path: String,
    pub format: String,
    pub memory_mb: f64,
}

// ── Handlers ────────────────────────────────────────────────────────

/// POST /infer — Run inference on the loaded quantized model.
pub async fn infer_handler(
    ctx: ServiceCtx,
    body: ShmSlice,
) -> HandlerResult<VilResponse<InferResponseBody>> {
    let svc = ctx
        .state::<QuantizedServiceState>()
        .expect("QuantizedServiceState");
    let runtime = &svc.runtime;
    let state = &svc.state;
    let req: InferRequest = body.json().expect("invalid JSON");
    if req.prompt.is_empty() {
        return Err(VilError::bad_request("prompt must not be empty"));
    }

    let rt = runtime
        .lock()
        .map_err(|e| VilError::internal(e.to_string()))?;

    if !rt.is_loaded() {
        return Err(VilError::bad_request(
            "model not loaded — call load() first",
        ));
    }

    let text = rt
        .generate(&req.prompt, req.max_tokens)
        .map_err(|e| VilError::internal(e))?;

    let event = QuantizeEvent {
        model_path: rt.config.path.clone(),
        format: format!("{}", rt.config.format),
        prompt_length: req.prompt.len(),
        max_tokens: req.max_tokens,
        memory_mb: rt.memory_estimate_mb(),
    };

    if let Ok(mut s) = state.lock() {
        s.record(&event);
    }

    Ok(VilResponse::ok(InferResponseBody {
        text,
        model_path: rt.config.path.clone(),
        format: format!("{}", rt.config.format),
        memory_mb: rt.memory_estimate_mb(),
    }))
}

/// GET /stats — Get quantized runtime statistics.
pub async fn stats_handler(ctx: ServiceCtx) -> VilResponse<QuantizedStatsBody> {
    let svc = ctx
        .state::<QuantizedServiceState>()
        .expect("QuantizedServiceState");
    let s = svc.state.lock().unwrap_or_else(|e| e.into_inner());
    VilResponse::ok(QuantizedStatsBody {
        total_inferences: s.total_inferences,
        total_errors: s.total_errors,
        total_tokens_generated: s.total_tokens_generated,
        model_loaded: s.model_loaded,
        model_path: s.model_path.clone(),
        format: s.format.clone(),
        memory_mb: s.memory_mb,
    })
}
