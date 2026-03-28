// =============================================================================
// VIL Server Capsule Handler — WASM hot-reload per route
// =============================================================================
//
// Handlers can be compiled to WASM and loaded into vil_capsule.
// This allows per-route hot-reload without restarting the server (~5ms).
//
// Architecture:
//   1. CapsuleRegistry holds loaded WASM modules per handler name
//   2. POST /admin/reload/{name} → reload a WASM module from disk
//   3. Incoming request → dispatch to capsule → return response
//   4. If capsule crashes, only that handler is affected
//
// Usage:
//   server.capsule_handler("/api/plugin", "plugin.wasm")
//   curl -X POST http://localhost:8080/admin/reload/plugin

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::routing::{get, post};
use axum::Router;
use dashmap::DashMap;
use std::sync::Arc;
use std::time::Instant;

use crate::state::AppState;

/// Registry of loaded WASM capsule handlers.
///
/// Each entry maps a handler name to its WASM module bytes.
/// Hot-reload swaps the bytes atomically — in-flight requests
/// finish on the old module, new requests use the new one.
pub struct CapsuleRegistry {
    /// Map of handler_name → WASM module bytes
    modules: DashMap<String, Arc<Vec<u8>>>,
    /// Map of handler_name → source file path (for reload)
    sources: DashMap<String, String>,
    /// Reload count per handler
    reload_counts: DashMap<String, u64>,
}

impl CapsuleRegistry {
    pub fn new() -> Self {
        Self {
            modules: DashMap::new(),
            sources: DashMap::new(),
            reload_counts: DashMap::new(),
        }
    }

    /// Load a WASM module from file and register it.
    pub fn load_from_file(&self, name: &str, path: &str) -> Result<(), String> {
        let bytes = std::fs::read(path)
            .map_err(|e| format!("Failed to read WASM file '{}': {}", path, e))?;

        self.modules.insert(name.to_string(), Arc::new(bytes));
        self.sources.insert(name.to_string(), path.to_string());
        self.reload_counts.insert(name.to_string(), 0);

        {
            use vil_log::app_log;
            app_log!(Info, "capsule.handler.loaded", { handler: name, path: path });
        }
        Ok(())
    }

    /// Load a WASM module from bytes.
    pub fn load_from_bytes(&self, name: &str, bytes: Vec<u8>) {
        let size = bytes.len();
        self.modules.insert(name.to_string(), Arc::new(bytes));
        self.reload_counts.insert(name.to_string(), 0);
        {
            use vil_log::app_log;
            app_log!(Info, "capsule.handler.loaded.bytes", { handler: name, size: size as u64 });
        }
    }

    /// Hot-reload a handler from its source file.
    /// Returns reload time in microseconds.
    pub fn reload(&self, name: &str) -> Result<u64, String> {
        let path = self.sources.get(name)
            .ok_or_else(|| format!("Handler '{}' has no source file", name))?
            .clone();

        let start = Instant::now();

        let bytes = std::fs::read(&path)
            .map_err(|e| format!("Failed to read '{}': {}", path, e))?;

        self.modules.insert(name.to_string(), Arc::new(bytes));

        let elapsed_us = start.elapsed().as_micros() as u64;

        // Increment reload count
        if let Some(mut count) = self.reload_counts.get_mut(name) {
            *count += 1;
        }

        {
            use vil_log::app_log;
            app_log!(Info, "capsule.handler.reloaded", { handler: name, reload_us: elapsed_us });
        }

        Ok(elapsed_us)
    }

    /// Get the WASM bytes for a handler.
    /// Get raw WASM module bytes for a handler.
    ///
    /// Callers use these bytes to create a CapsuleHost for execution.
    /// See wasm_dispatch::invoke_wasm_capsule() for Level 1 zero-copy dispatch.
    pub fn get_module(&self, name: &str) -> Option<Arc<Vec<u8>>> {
        self.modules.get(name).map(|v| v.value().clone())
    }

    /// Check if a handler is loaded.
    pub fn is_loaded(&self, name: &str) -> bool {
        self.modules.contains_key(name)
    }

    /// Get all loaded handler names.
    pub fn loaded_handlers(&self) -> Vec<String> {
        self.modules.iter().map(|e| e.key().clone()).collect()
    }

    /// Get reload count for a handler.
    pub fn reload_count(&self, name: &str) -> u64 {
        self.reload_counts.get(name).map(|v| *v).unwrap_or(0)
    }

    /// Get the number of loaded handlers.
    pub fn handler_count(&self) -> usize {
        self.modules.len()
    }
}

impl Default for CapsuleRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Create the admin router for capsule management.
///
/// Endpoints:
///   POST /admin/reload/{name}  → hot-reload a WASM handler
///   GET  /admin/capsules       → list all loaded capsule handlers
pub fn capsule_admin_router() -> Router<AppState> {
    Router::new()
        .route("/admin/reload/:name", post(reload_handler))
        .route("/admin/capsules", get(list_capsules))
}

/// Hot-reload a WASM handler by name.
async fn reload_handler(
    State(state): State<AppState>,
    Path(name): Path<String>,
) -> impl IntoResponse {
    let registry = state.capsule_registry();

    if !registry.is_loaded(&name) {
        return (
            StatusCode::NOT_FOUND,
            axum::Json(serde_json::json!({
                "error": format!("Handler '{}' not found", name),
                "loaded_handlers": registry.loaded_handlers(),
            })),
        );
    }

    match registry.reload(&name) {
        Ok(elapsed_us) => (
            StatusCode::OK,
            axum::Json(serde_json::json!({
                "handler": name,
                "status": "reloaded",
                "reload_time_us": elapsed_us,
                "total_reloads": registry.reload_count(&name),
            })),
        ),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            axum::Json(serde_json::json!({
                "handler": name,
                "error": e,
            })),
        ),
    }
}

/// List all loaded capsule handlers.
async fn list_capsules(
    State(state): State<AppState>,
) -> impl IntoResponse {
    let registry = state.capsule_registry();
    let handlers: Vec<serde_json::Value> = registry.loaded_handlers().iter().map(|name| {
        serde_json::json!({
            "name": name,
            "loaded": true,
            "reloads": registry.reload_count(name),
            "has_source": registry.modules.contains_key(name),
        })
    }).collect();

    axum::Json(serde_json::json!({
        "capsule_handlers": handlers,
        "total": registry.handler_count(),
    }))
}
