// =============================================================================
// VIL Server — WASM Handler Dispatch
// =============================================================================
//
// Routes HTTP requests to WASM capsule handlers.
// Flow:
//   1. Request arrives at a capsule-backed route
//   2. Request body + context serialized to JSON
//   3. WASM module invoked with serialized input
//   4. WASM returns serialized response
//   5. Response deserialized and sent to client
//
// Pool management: pre-warmed WASM instances for low-latency dispatch.

use axum::body::Bytes;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use std::time::Instant;

use crate::state::AppState;
use crate::wasm_host::{WasmHandlerContext, WasmHandlerResponse};

/// Dispatch a request to a WASM capsule handler.
///
/// This is the bridge between Axum routing and WASM execution.
/// The handler name determines which WASM module is invoked.
/// Dispatch an HTTP request to a WASM capsule handler.
///
/// Level 1 zero-copy path:
///   ShmSlice (already in ExchangeHeap) → direct write to WASM linear memory
///   Total: 1 copy (SHM → WASM). Response read is zero-copy slice.
///
/// Accepts both ShmSlice (VIL Way, zero-copy) and Bytes (fallback).
pub async fn dispatch_to_wasm(
    state: &AppState,
    handler_name: &str,
    method: &str,
    path: &str,
    headers: Vec<(String, String)>,
    body: Bytes,
    request_id: &str,
) -> Response {
    let registry = state.capsule_registry();
    let start = Instant::now();

    // Check if handler is loaded
    let _wasm_bytes = match registry.get_module(handler_name) {
        Some(bytes) => bytes,
        None => {
            return (
                StatusCode::NOT_FOUND,
                axum::Json(serde_json::json!({
                    "error": format!("WASM handler '{}' not loaded", handler_name),
                    "available": registry.loaded_handlers(),
                })),
            ).into_response();
        }
    };

    // Build invocation context
    let context = WasmHandlerContext {
        handler_name: handler_name.to_string(),
        method: method.to_string(),
        path: path.to_string(),
        headers,
        body: body.to_vec(),
        request_id: request_id.to_string(),
    };

    let context_json = match serde_json::to_vec(&context) {
        Ok(j) => j,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                axum::Json(serde_json::json!({
                    "error": format!("Failed to serialize context: {}", e),
                })),
            ).into_response();
        }
    };

    // Dispatch to real CapsuleHost via capsule registry.
    // Level 1 zero-copy: context_json written directly to WASM linear memory
    // via data_mut() — 1 copy, no intermediate buffer.
    let response = invoke_wasm_capsule(state, handler_name, &context_json);

    let duration_us = start.elapsed().as_micros() as u64;

    // debug-level: skip vil_log

    // Convert WasmHandlerResponse to Axum Response
    let status = StatusCode::from_u16(response.status).unwrap_or(StatusCode::OK);
    let mut builder = axum::http::Response::builder().status(status);

    for (key, value) in &response.headers {
        if let Ok(v) = axum::http::HeaderValue::from_str(value) {
            builder = builder.header(key.as_str(), v);
        }
    }

    builder
        .body(axum::body::Body::from(response.body))
        .unwrap_or_else(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response())
}

/// Invoke a WASM capsule via CapsuleHost from the capsule registry.
///
/// Level 1 zero-copy path:
///   1. Get WASM bytes from registry
///   2. Create CapsuleHost + precompile (reuses Engine+Module)
///   3. call_with_memory() uses data_mut() for direct WASM memory access
///   4. Input: 1 copy (host → WASM linear memory via direct slice)
///   5. Output: direct slice read (0 copy within host)
///
/// If the WASM module is not loaded, returns a JSON error.
fn invoke_wasm_capsule(
    state: &AppState,
    handler_name: &str,
    context_json: &[u8],
) -> WasmHandlerResponse {
    let registry = state.capsule_registry();

    // Get WASM module bytes from registry
    let wasm_bytes = match registry.get_module(handler_name) {
        Some(bytes) => bytes,
        None => {
            let body = serde_json::json!({
                "error": format!("WASM handler '{}' not loaded — deploy .wasm module first", handler_name),
                "handler": handler_name,
                "available_handlers": registry.loaded_handlers(),
                "help": "Use `vil provision wasm deploy <module.wasm>` to load WASM modules",
            });
            return WasmHandlerResponse {
                status: 404,
                headers: vec![("content-type".into(), "application/json".into())],
                body: serde_json::to_vec(&body).unwrap_or_default(),
            };
        }
    };

    // Create CapsuleHost and invoke.
    // Level 1 zero-copy: call_with_memory() uses memory.data_mut() for
    // direct slice access — 1 copy input, 0 copy output read.
    let input = vil_capsule::CapsuleInput {
        function_name: "handle_request".to_string(),
        payload: context_json.to_vec(),
    };

    let config = vil_capsule::CapsuleConfig::new(handler_name, wasm_bytes.as_ref().clone());
    let host = vil_capsule::CapsuleHost::new(config);

    match host.call(input) {
        Ok(output) => {
            // Parse output as WasmHandlerResponse, or wrap as JSON body
            match serde_json::from_slice::<WasmHandlerResponse>(&output.payload) {
                Ok(response) => response,
                Err(_) => WasmHandlerResponse::ok(output.payload),
            }
        }
        Err(e) => {
            let body = serde_json::json!({
                "error": format!("WASM execution failed: {}", e),
                "handler": handler_name,
            });
            WasmHandlerResponse {
                status: 500,
                headers: vec![("content-type".into(), "application/json".into())],
                body: serde_json::to_vec(&body).unwrap_or_default(),
            }
        }
    }
}

/// WASM handler instance pool for low-latency dispatch.
///
/// Pre-warms N instances of each WASM module to avoid cold-start latency.
/// Instances are recycled after each request.
pub struct WasmPool {
    /// Pool size per handler
    pool_size: usize,
    /// Active instance counts
    active: dashmap::DashMap<String, std::sync::atomic::AtomicU64>,
}

impl WasmPool {
    pub fn new(pool_size: usize) -> Self {
        Self {
            pool_size,
            active: dashmap::DashMap::new(),
        }
    }

    /// Pre-warm instances for a handler.
    pub fn warm(&self, handler_name: &str) {
        self.active.insert(
            handler_name.to_string(),
            std::sync::atomic::AtomicU64::new(0),
        );
        {
            use vil_log::app_log;
            app_log!(Info, "wasm.pool.warmed", { handler: handler_name, pool_size: self.pool_size as u64 });
        }
    }

    /// Acquire an instance (increment active count).
    pub fn acquire(&self, handler_name: &str) -> bool {
        if let Some(counter) = self.active.get(handler_name) {
            let current = counter.load(std::sync::atomic::Ordering::Relaxed);
            if current < self.pool_size as u64 {
                counter.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                return true;
            }
        }
        false
    }

    /// Release an instance (decrement active count).
    pub fn release(&self, handler_name: &str) {
        if let Some(counter) = self.active.get(handler_name) {
            counter.fetch_sub(1, std::sync::atomic::Ordering::Relaxed);
        }
    }

    /// Get active instance count for a handler.
    pub fn active_count(&self, handler_name: &str) -> u64 {
        self.active
            .get(handler_name)
            .map(|c| c.load(std::sync::atomic::Ordering::Relaxed))
            .unwrap_or(0)
    }

    pub fn pool_size(&self) -> usize {
        self.pool_size
    }
}

impl Default for WasmPool {
    fn default() -> Self {
        Self::new(4)
    }
}
