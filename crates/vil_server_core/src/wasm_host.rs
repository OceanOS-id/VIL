// =============================================================================
// VIL Server — WASM Host Function Registry
// =============================================================================
//
// Extends vil_capsule with server-specific host functions that WASM
// handlers can call:
//   - vil_log(ptr, len)       → log a message from WASM
//   - vil_http_status(code)   → set response status code
//   - vil_set_header(k, v)    → set response header
//   - vil_read_body(ptr, len) → read request body into WASM memory
//   - vil_shm_read(region, offset, len) → read from SHM
//   - vil_shm_write(region, offset, ptr, len) → write to SHM
//   - vil_metric_inc(name)    → increment a counter
//
// These form the "capability API" for WASM handlers — controlled
// access to server features without full native privileges.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Host function capability — what a WASM handler is allowed to do.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum WasmCapability {
    /// Log messages
    Log,
    /// Set HTTP response status/headers
    HttpResponse,
    /// Read request body
    ReadBody,
    /// Read from SHM regions
    ShmRead,
    /// Write to SHM regions
    ShmWrite,
    /// Increment metrics counters
    Metrics,
    /// Access mesh (send to other services)
    MeshSend,
    /// Access key-value store
    KvStore,
}

/// WASM host function registry — defines what functions are available
/// to WASM handlers at runtime.
pub struct WasmHostRegistry {
    /// Enabled capabilities per handler
    capabilities: HashMap<String, Vec<WasmCapability>>,
    /// Default capabilities for new handlers
    default_capabilities: Vec<WasmCapability>,
}

impl WasmHostRegistry {
    pub fn new() -> Self {
        Self {
            capabilities: HashMap::new(),
            default_capabilities: vec![
                WasmCapability::Log,
                WasmCapability::HttpResponse,
                WasmCapability::ReadBody,
                WasmCapability::Metrics,
            ],
        }
    }

    /// Set capabilities for a specific handler.
    pub fn set_capabilities(&mut self, handler: &str, caps: Vec<WasmCapability>) {
        self.capabilities.insert(handler.to_string(), caps);
    }

    /// Grant additional capability to a handler.
    pub fn grant(&mut self, handler: &str, cap: WasmCapability) {
        self.capabilities
            .entry(handler.to_string())
            .or_insert_with(|| self.default_capabilities.clone())
            .push(cap);
    }

    /// Check if a handler has a specific capability.
    pub fn has_capability(&self, handler: &str, cap: WasmCapability) -> bool {
        self.capabilities
            .get(handler)
            .unwrap_or(&self.default_capabilities)
            .contains(&cap)
    }

    /// Get all capabilities for a handler.
    pub fn get_capabilities(&self, handler: &str) -> &[WasmCapability] {
        self.capabilities
            .get(handler)
            .map(|v| v.as_slice())
            .unwrap_or(&self.default_capabilities)
    }

    /// Set default capabilities for new handlers.
    pub fn set_defaults(&mut self, caps: Vec<WasmCapability>) {
        self.default_capabilities = caps;
    }
}

impl Default for WasmHostRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// WASM handler invocation context — passed to the WASM runtime
/// during handler execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WasmHandlerContext {
    /// Handler name
    pub handler_name: String,
    /// Request method
    pub method: String,
    /// Request path
    pub path: String,
    /// Request headers (flattened)
    pub headers: Vec<(String, String)>,
    /// Request body bytes
    pub body: Vec<u8>,
    /// Request ID
    pub request_id: String,
}

/// WASM handler response — returned from WASM execution.
#[derive(Debug, Clone, Deserialize)]
pub struct WasmHandlerResponse {
    /// HTTP status code
    pub status: u16,
    /// Response headers
    pub headers: Vec<(String, String)>,
    /// Response body
    pub body: Vec<u8>,
}

impl WasmHandlerResponse {
    pub fn ok(body: Vec<u8>) -> Self {
        Self {
            status: 200,
            headers: vec![("content-type".to_string(), "application/json".to_string())],
            body,
        }
    }

    pub fn error(status: u16, message: &str) -> Self {
        let body = serde_json::json!({
            "error": message,
            "status": status,
        });
        Self {
            status,
            headers: vec![("content-type".to_string(), "application/json".to_string())],
            body: serde_json::to_vec(&body).unwrap_or_default(),
        }
    }
}
