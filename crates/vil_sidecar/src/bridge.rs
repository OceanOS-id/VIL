// =============================================================================
// Sidecar Bridge — Zero-plumbing runtime for #[vil_sidecar] macro
// =============================================================================
// Developer NEVER touches this. The #[vil_sidecar] macro generates code that
// calls these functions. Registry, connection, spawning — all automatic.
//
// Fallback: if sidecar process is not available (not running, not connected),
// the bridge returns None — macro-generated code falls back to native Rust body.

use std::sync::{Arc, OnceLock};
use std::sync::atomic::{AtomicBool, Ordering};

use crate::{dispatcher, DispatchError, InvokeResponse, SidecarConfig, SidecarRegistry};

/// Global sidecar registry — lazy initialized on first use.
static SIDECAR_REGISTRY: OnceLock<Arc<SidecarRegistry>> = OnceLock::new();
static SIDECAR_FALLBACK_WARNED: AtomicBool = AtomicBool::new(false);

fn global_registry() -> &'static Arc<SidecarRegistry> {
    SIDECAR_REGISTRY.get_or_init(|| Arc::new(SidecarRegistry::new()))
}

fn warn_fallback(target: &str, method: &str) {
    if !SIDECAR_FALLBACK_WARNED.swap(true, Ordering::Relaxed) {
        eprintln!(
            "[VIL] Sidecar '{}' not available — running {}.{}() as native Rust fallback. \
             Start sidecar process for SHM+UDS execution.",
            target, target, method
        );
    }
}

/// Ensure a sidecar target is registered. Lazy — registers on first call.
pub fn ensure_target(target_name: &str, source_file: &str, timeout_ms: u64) {
    let registry = global_registry();
    if registry.get(target_name).is_some() {
        return;
    }
    let command = if source_file.ends_with(".py") {
        format!("python3 {}", source_file)
    } else if source_file.ends_with(".go") {
        format!("go run {}", source_file)
    } else if source_file.ends_with(".js") || source_file.ends_with(".ts") {
        format!("node {}", source_file)
    } else if !source_file.is_empty() {
        source_file.to_string()
    } else {
        target_name.to_string()
    };
    let config = SidecarConfig::new(target_name)
        .command(command)
        .timeout(timeout_ms);
    registry.register(config);
}

/// Try to call sidecar. Returns None if unavailable (native fallback).
pub async fn try_call_sidecar<T: serde::de::DeserializeOwned>(
    target: &str,
    method: &str,
    input: &[u8],
) -> Option<T> {
    let registry = global_registry();
    let resp = dispatcher::invoke(registry, target, method, input).await.ok()?;
    serde_json::from_slice(&resp.data).ok()
}

/// Bridge: call sidecar and deserialize. Panics only if sidecar IS available but fails.
/// Returns native fallback signal if sidecar unavailable.
pub async fn call_sidecar<T: serde::de::DeserializeOwned>(
    target: &str,
    method: &str,
    input: &[u8],
) -> T {
    let registry = global_registry();
    match dispatcher::invoke(registry, target, method, input).await {
        Ok(resp) => {
            serde_json::from_slice(&resp.data)
                .unwrap_or_else(|e| panic!("Sidecar {}.{}() deserialize: {}", target, method, e))
        }
        Err(e) => {
            warn_fallback(target, method);
            panic!("Sidecar {}.{}() failed: {} — use try_call_sidecar for fallback", target, method, e)
        }
    }
}

/// Bridge: call sidecar, return raw bytes.
pub async fn call_sidecar_raw(
    target: &str,
    method: &str,
    input: &[u8],
) -> Result<InvokeResponse, DispatchError> {
    let registry = global_registry();
    dispatcher::invoke(registry, target, method, input).await
}

/// Check if sidecar is available (registered + healthy).
pub fn sidecar_available(target: &str) -> bool {
    let registry = global_registry();
    registry.get(target).is_some()
}

/// Metadata emitted by #[vil_sidecar] for introspection.
#[derive(Debug, Clone)]
pub struct SidecarFnMeta {
    pub target_name: &'static str,
    pub method_name: &'static str,
    pub source_file: &'static str,
    pub timeout_ms: u64,
}
