// =============================================================================
// vil_vwfd::handler_provision — Auto-provision handlers on workflow upload
// =============================================================================
//
// When a VWFD YAML is compiled to VilwGraph, this module scans all activities
// and auto-registers missing handlers:
//
//   NativeCode → load {PLUGIN_DIR}/{handler_ref}.so via dlopen
//   Function   → load {WASM_DIR}/{module_ref}.wasm via wasmtime
//   Sidecar    → spawn process from sidecar_config.command
//
// Priority on upload:
//   1. If file exists in dir → ALWAYS load (update/replace existing)
//   2. If no file but handler already registered → keep existing (no-op)
//   3. If no file and not registered → report missing
//
// Compatible with vflow_plugin_sdk ABI.

use std::sync::Arc;
use crate::graph::{VilwGraph, NodeKind};

/// Result of auto-provisioning scan.
#[derive(Debug, Default)]
pub struct ProvisionResult {
    pub provisioned: Vec<String>,
    pub missing: Vec<String>,
    pub errors: Vec<String>,
}

/// Scan a compiled VilwGraph and auto-provision handlers.
///
/// Priority: new file in dir → load/update. No file → use existing. Neither → missing.
pub fn provision_handlers(
    graph: &VilwGraph,
    plugin_registry: &crate::plugin_loader::PluginRegistry,
    #[cfg(feature = "wasm")]
    wasm_registry: &Arc<std::sync::RwLock<std::collections::HashMap<String, Arc<crate::app::WasmWorkerPool>>>>,
    sidecar_pool: &Arc<std::sync::RwLock<crate::app::SidecarPool>>,
) -> ProvisionResult {
    let plugin_dir = std::env::var("VIL_PLUGIN_DIR")
        .unwrap_or_else(|_| "/var/lib/vil/plugins".to_string());
    #[cfg(feature = "wasm")]
    let wasm_dir = std::env::var("VIL_WASM_DIR")
        .unwrap_or_else(|_| "/var/lib/vil/modules".to_string());

    let mut result = ProvisionResult::default();

    for node in &graph.nodes {
        match node.kind {
            NodeKind::NativeCode => {
                let handler_ref = node.config.get("handler_ref")
                    .and_then(|v| v.as_str()).unwrap_or("");
                if handler_ref.is_empty() { continue; }

                // Priority: new .so file first (update), then existing
                let so_path = std::path::Path::new(&plugin_dir)
                    .join(format!("{}.so", handler_ref));
                if so_path.exists() {
                    // Always load — overwrites existing (versioning via file replace)
                    let was_existing = plugin_registry.has(handler_ref);
                    match plugin_registry.load(&so_path) {
                        Ok(name) => {
                            let action = if was_existing { "updated" } else { "loaded" };
                            tracing::info!("Plugin {}: {} (from {})", action, name, so_path.display());
                            result.provisioned.push(format!("code.{}", name));
                        }
                        Err(e) => {
                            tracing::warn!("Plugin load '{}' failed: {}", handler_ref, e);
                            result.errors.push(format!("code.{}: {}", handler_ref, e));
                        }
                    }
                } else if !plugin_registry.has(handler_ref) {
                    // No file, no existing handler → missing
                    result.missing.push(format!("code.{} (need {}.so in {})", handler_ref, handler_ref, plugin_dir));
                }
                // else: no new file but handler exists → keep existing
            }

            #[cfg(feature = "wasm")]
            NodeKind::Function => {
                let module_ref = node.config.get("module_ref")
                    .and_then(|v| v.as_str()).unwrap_or("");
                if module_ref.is_empty() { continue; }

                // Priority: new .wasm file first (update), then existing
                let wasm_path = std::path::Path::new(&wasm_dir)
                    .join(format!("{}.wasm", module_ref));
                if wasm_path.exists() {
                    // Always load — hot-swap WASM module
                    match register_wasm_from_file(wasm_registry, &wasm_path, module_ref) {
                        Ok(()) => {
                            tracing::info!("WASM loaded/updated: {} (from {})", module_ref, wasm_path.display());
                            result.provisioned.push(format!("wasm.{}", module_ref));
                        }
                        Err(e) => {
                            tracing::warn!("WASM load '{}' failed: {}", module_ref, e);
                            result.errors.push(format!("wasm.{}: {}", module_ref, e));
                        }
                    }
                } else {
                    let exists = wasm_registry.read().unwrap().contains_key(module_ref);
                    if !exists {
                        result.missing.push(format!("wasm.{} (need {}.wasm in {})", module_ref, module_ref, wasm_dir));
                    }
                }
            }

            NodeKind::Sidecar => {
                let target = node.config.get("target")
                    .and_then(|v| v.as_str()).unwrap_or("");
                if target.is_empty() { continue; }

                let command = node.config.get("command")
                    .and_then(|v| v.as_str()).unwrap_or("");
                if command.is_empty() {
                    let exists = sidecar_pool.read().unwrap().has(target);
                    if !exists {
                        result.missing.push(format!("sidecar.{} (no command in config, not registered)", target));
                    }
                    continue;
                }

                // Always register/update — new command replaces old pool
                {
                    let mut pool = sidecar_pool.write().unwrap();
                    pool.register(target.to_string(), command.to_string());
                }
                tracing::info!("Sidecar registered: {} (command='{}')", target, command);
                result.provisioned.push(format!("sidecar.{}", target));
            }

            _ => {}
        }
    }

    if !result.provisioned.is_empty() {
        tracing::info!("Provisioned {} handlers: {:?}", result.provisioned.len(), result.provisioned);
    }
    if !result.missing.is_empty() {
        tracing::warn!("Missing {} handlers: {:?}", result.missing.len(), result.missing);
    }

    result
}

/// Register a WASM module from file into the shared wasm registry at runtime.
/// Also exposed as `register_wasm_from_file` for admin API upload.
#[cfg(feature = "wasm")]
pub fn register_wasm_from_file(
    registry: &Arc<std::sync::RwLock<std::collections::HashMap<String, Arc<crate::app::WasmWorkerPool>>>>,
    wasm_path: &std::path::Path,
    module_ref: &str,
) -> Result<(), String> {
    let bytes = std::fs::read(wasm_path)
        .map_err(|e| format!("read {}: {}", wasm_path.display(), e))?;

    let mut wasm_cfg = wasmtime::Config::new();
    wasm_cfg.cranelift_opt_level(wasmtime::OptLevel::Speed);
    wasm_cfg.parallel_compilation(true);

    let engine = Arc::new(wasmtime::Engine::new(&wasm_cfg)
        .map_err(|e| format!("wasmtime engine: {}", e))?);

    let module = wasmtime::Module::new(&engine, &bytes)
        .map_err(|e| format!("wasm compile {}: {}", module_ref, e))?;

    let mut linker = wasmtime::Linker::<wasmtime_wasi::preview1::WasiP1Ctx>::new(&engine);
    wasmtime_wasi::preview1::add_to_linker_sync(&mut linker, |ctx| ctx)
        .map_err(|e| format!("wasm link {}: {}", module_ref, e))?;

    let instance_pre = linker.instantiate_pre(&module)
        .map_err(|e| format!("wasm pre-instantiate {}: {}", module_ref, e))?;

    let num_workers = std::thread::available_parallelism()
        .map(|n| n.get()).unwrap_or(4).min(8);

    let pool = crate::app::WasmWorkerPool::new(engine, Arc::new(instance_pre), num_workers);

    registry.write().unwrap().insert(module_ref.to_string(), Arc::new(pool));
    Ok(())
}
