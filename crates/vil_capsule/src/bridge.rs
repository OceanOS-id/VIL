// =============================================================================
// WASM Bridge — Zero-plumbing runtime for #[vil_wasm] macro
// =============================================================================
// Developer NEVER touches this. The #[vil_wasm] macro generates code that
// calls these functions. Registry, pool, module loading — all automatic.
//
// Fallback: if WASM module is not available (not built, feature not enabled),
// the bridge returns a sentinel value indicating native fallback should be used.
// The macro-generated code calls the preserved __vil_wasm_body_{fn} in that case.

use std::sync::{Arc, OnceLock};
use std::sync::atomic::{AtomicBool, Ordering};

use crate::{WasmFaaSConfig, WasmFaaSRegistry, WasmPool};

/// Global WASM registry — lazy initialized on first use.
static WASM_REGISTRY: OnceLock<Arc<WasmFaaSRegistry>> = OnceLock::new();
static WASM_FALLBACK_WARNED: AtomicBool = AtomicBool::new(false);

fn global_registry() -> &'static Arc<WasmFaaSRegistry> {
    WASM_REGISTRY.get_or_init(|| Arc::new(WasmFaaSRegistry::new()))
}

fn warn_fallback(module: &str, function: &str) {
    if !WASM_FALLBACK_WARNED.swap(true, Ordering::Relaxed) {
        eprintln!(
            "[VIL] WASM module '{}' not available — running {}.{}() as native Rust fallback. \
             Build WASM modules or enable --features wasm for sandboxed execution.",
            module, module, function
        );
    }
}

/// Resolve WASM module file from conventional paths.
fn resolve_wasm_path(module_name: &str) -> Option<Vec<u8>> {
    let mut candidates = Vec::new();
    if let Ok(dir) = std::env::var("VIL_WASM_DIR") {
        candidates.push(format!("{}/{}.wasm", dir, module_name));
    }
    candidates.extend([
        format!("wasm-modules/out/{}.wasm", module_name),
        format!("wasm-modules/{}.wasm", module_name),
        format!("{}.wasm", module_name),
    ]);
    for path in &candidates {
        if let Ok(bytes) = std::fs::read(path) {
            return Some(bytes);
        }
    }
    None
}

/// Try to ensure a WASM module pool. Returns None if unavailable (fallback to native).
fn try_ensure_pool(module_name: &str) -> Option<Arc<WasmPool>> {
    let registry = global_registry();
    if let Some(pool) = registry.get(module_name) {
        return Some(pool);
    }
    let wasm_bytes = resolve_wasm_path(module_name)?;
    let config = WasmFaaSConfig::new(module_name, wasm_bytes)
        .pool_size(4)
        .timeout_ms(5000);
    Some(registry.register(config))
}

/// Override pool config for a module.
pub fn configure_pool(module_name: &str, pool_size: usize, timeout_ms: u64) {
    let registry = global_registry();
    if registry.get(module_name).is_some() {
        return;
    }
    if let Some(wasm_bytes) = resolve_wasm_path(module_name) {
        let config = WasmFaaSConfig::new(module_name, wasm_bytes)
            .pool_size(pool_size)
            .timeout_ms(timeout_ms);
        registry.register(config);
    }
}

/// Check if WASM execution is available for a module.
/// Used by macro-generated code to decide: WASM bridge or native fallback.
pub fn wasm_available(module: &str) -> bool {
    try_ensure_pool(module).is_some()
}

/// Bridge: i32 x i32 → i32. Returns None if WASM unavailable (native fallback).
#[cfg(feature = "wasm")]
pub fn try_call_wasm_i32(module: &str, function: &str, arg0: i32, arg1: i32) -> Option<i32> {
    let pool = try_ensure_pool(module)?;
    Some(pool.call_i32(function, arg0, arg1).ok()?)
}

#[cfg(not(feature = "wasm"))]
pub fn try_call_wasm_i32(_module: &str, _function: &str, _arg0: i32, _arg1: i32) -> Option<i32> {
    None
}

/// Bridge: i32 call with native fallback.
/// Called by #[vil_wasm] generated code. Falls back to native if WASM unavailable.
pub fn call_wasm_i32(module: &str, function: &str, arg0: i32, arg1: i32) -> i32 {
    if let Some(result) = try_call_wasm_i32(module, function, arg0, arg1) {
        return result;
    }
    warn_fallback(module, function);
    // Return sentinel — macro-generated code detects this and calls native body
    i32::MIN // sentinel: native fallback needed
}

/// Bridge: bytes → bytes. Returns None if WASM unavailable.
#[cfg(feature = "wasm")]
pub fn try_call_wasm_memory(module: &str, function: &str, input: &[u8]) -> Option<Vec<u8>> {
    let pool = try_ensure_pool(module)?;
    Some(pool.call_with_memory(function, input).ok()?)
}

#[cfg(not(feature = "wasm"))]
pub fn try_call_wasm_memory(_module: &str, _function: &str, _input: &[u8]) -> Option<Vec<u8>> {
    None
}

pub fn call_wasm_memory(module: &str, function: &str, input: &[u8]) -> Vec<u8> {
    if let Some(result) = try_call_wasm_memory(module, function, input) {
        return result;
    }
    warn_fallback(module, function);
    Vec::new() // empty = native fallback needed
}

/// Metadata emitted by #[vil_wasm] for introspection.
#[derive(Debug, Clone)]
pub struct WasmFnMeta {
    pub module_name: &'static str,
    pub function_name: &'static str,
    pub pool_size: usize,
    pub timeout_ms: u64,
}
