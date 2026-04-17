// =============================================================================
// vil_vwfd::plugin_loader — .so Plugin Loader (dlopen)
// =============================================================================
//
// Loads NativeCode handlers from shared libraries (.so/.dylib) at runtime.
// ABI: 3 C functions per plugin (compatible with vflow_plugin_sdk):
//   - vflow_plugin_name() -> *const c_char
//   - vflow_plugin_execute(in_ptr, in_len, out_ptr, out_len) -> i32
//   - vflow_plugin_free(ptr, len)
//
// Performance: A persistent multi-threaded tokio runtime is shared across all
// plugin calls, eliminating per-call thread spawn + runtime creation overhead.

use serde_json::Value;
use std::collections::HashMap;
use std::ffi::CStr;
use std::path::Path;
use std::sync::{Arc, RwLock};

/// Loaded plugin handle — keeps library alive via _lib ownership.
struct PluginHandle {
    _lib: libloading::Library,
    name: String,
    execute_fn: unsafe extern "C" fn(*const u8, usize, *mut *mut u8, *mut usize) -> i32,
    free_fn: unsafe extern "C" fn(*mut u8, usize),
}

// SAFETY: Plugin functions are required to be thread-safe by ABI contract.
unsafe impl Send for PluginHandle {}
unsafe impl Sync for PluginHandle {}

impl PluginHandle {
    /// Load a plugin from a .so/.dylib path.
    fn load(path: &Path) -> Result<Self, String> {
        unsafe {
            let lib = libloading::Library::new(path)
                .map_err(|e| format!("dlopen '{}': {}", path.display(), e))?;

            let name_fn: libloading::Symbol<unsafe extern "C" fn() -> *const std::ffi::c_char> =
                lib.get(b"vflow_plugin_name")
                    .map_err(|e| format!("symbol vflow_plugin_name: {}", e))?;
            let name_ptr = name_fn();
            let name = CStr::from_ptr(name_ptr)
                .to_str()
                .map_err(|_| "plugin name: invalid UTF-8".to_string())?
                .to_string();

            let execute_fn: libloading::Symbol<unsafe extern "C" fn(*const u8, usize, *mut *mut u8, *mut usize) -> i32> =
                lib.get(b"vflow_plugin_execute")
                    .map_err(|e| format!("symbol vflow_plugin_execute: {}", e))?;
            let execute_fn = *execute_fn;

            let free_fn: libloading::Symbol<unsafe extern "C" fn(*mut u8, usize)> =
                lib.get(b"vflow_plugin_free")
                    .map_err(|e| format!("symbol vflow_plugin_free: {}", e))?;
            let free_fn = *free_fn;

            Ok(Self { _lib: lib, name, execute_fn, free_fn })
        }
    }

    /// Execute the plugin: JSON bytes in → JSON bytes out.
    fn execute(&self, input: &[u8]) -> Result<Vec<u8>, String> {
        let mut out_ptr: *mut u8 = std::ptr::null_mut();
        let mut out_len: usize = 0;

        let rc = unsafe {
            (self.execute_fn)(input.as_ptr(), input.len(), &mut out_ptr, &mut out_len)
        };

        if rc != 0 {
            if !out_ptr.is_null() && out_len > 0 {
                let msg = unsafe {
                    let slice = std::slice::from_raw_parts(out_ptr, out_len);
                    let s = String::from_utf8_lossy(slice).to_string();
                    (self.free_fn)(out_ptr, out_len);
                    s
                };
                return Err(format!("plugin '{}': {}", self.name, msg));
            }
            return Err(format!("plugin '{}': error code {}", self.name, rc));
        }

        if out_ptr.is_null() || out_len == 0 {
            return Ok(b"null".to_vec());
        }

        let result = unsafe {
            let slice = std::slice::from_raw_parts(out_ptr, out_len);
            let vec = slice.to_vec();
            (self.free_fn)(out_ptr, out_len);
            vec
        };

        Ok(result)
    }
}

/// Thread-safe registry of dynamically loaded .so plugins.
/// Supports hot-load: new plugins can be added at runtime without restart.
///
/// Uses a persistent tokio runtime for .so dispatch — handlers that call
/// tokio::task::block_in_place or Handle::current() get a proper context
/// without per-call thread spawn + runtime creation overhead.
pub struct PluginRegistry {
    plugins: RwLock<HashMap<String, Arc<PluginHandle>>>,
    /// Persistent multi-threaded tokio runtime for .so handler execution.
    /// Shared across all calls — eliminates 30ms per-call overhead.
    plugin_rt: Arc<tokio::runtime::Runtime>,
}

impl PluginRegistry {
    pub fn new() -> Self {
        let workers = std::thread::available_parallelism()
            .map(|n| n.get()).unwrap_or(4).min(8);
        let plugin_rt = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(workers)
            .thread_name("vil-plugin")
            .enable_all()
            .build()
            .expect("plugin runtime");
        Self {
            plugins: RwLock::new(HashMap::new()),
            plugin_rt: Arc::new(plugin_rt),
        }
    }

    /// Load a .so plugin and register by its embedded name.
    /// Returns the handler name. Overwrites existing handler with same name (versioning).
    pub fn load(&self, path: &Path) -> Result<String, String> {
        let handle = Arc::new(PluginHandle::load(path)?);
        let name = handle.name.clone();
        self.plugins.write().unwrap().insert(name.clone(), handle);
        tracing::info!("Plugin loaded: {} (from {})", name, path.display());
        Ok(name)
    }

    /// Dispatch to a loaded plugin by handler_ref name.
    /// Uses the persistent tokio runtime's blocking thread pool — no per-call
    /// thread spawn or runtime creation. Handlers get full tokio context
    /// (block_in_place, Handle::current, spawn_blocking all work).
    pub fn call(&self, handler_ref: &str, input: &Value) -> Result<Value, String> {
        let plugins = self.plugins.read().unwrap();
        let handle = plugins.get(handler_ref)
            .ok_or_else(|| format!("plugin '{}' not loaded", handler_ref))?
            .clone();
        drop(plugins);

        let input_bytes = serde_json::to_vec(input).unwrap_or_else(|_| b"null".to_vec());

        // Dispatch on the persistent runtime's blocking thread pool via oneshot channel.
        // Cannot use rt.block_on() because caller is already inside a tokio runtime.
        // spawn_blocking reuses threads from the pool — no spawn overhead after warmup.
        let (tx, rx) = std::sync::mpsc::sync_channel::<Result<Vec<u8>, String>>(1);
        let rt = self.plugin_rt.clone();
        let href = handler_ref.to_string();
        rt.spawn_blocking(move || {
            let result = handle.execute(&input_bytes);
            let _ = tx.send(result);
        });
        let result = rx.recv()
            .map_err(|_| format!("plugin '{}' channel closed", href))??;

        serde_json::from_slice(&result)
            .map_err(|e| format!("plugin '{}' output: {}", handler_ref, e))
    }

    /// Check if a handler is loaded.
    pub fn has(&self, handler_ref: &str) -> bool {
        self.plugins.read().unwrap().contains_key(handler_ref)
    }

    /// List all loaded handler names.
    pub fn names(&self) -> Vec<String> {
        self.plugins.read().unwrap().keys().cloned().collect()
    }

    /// Scan a directory for .so/.dylib files and load all.
    pub fn scan_dir(&self, dir: &Path) -> Vec<String> {
        let mut loaded = Vec::new();
        let entries = match std::fs::read_dir(dir) {
            Ok(e) => e,
            Err(e) => {
                tracing::debug!("Plugin dir '{}' not readable: {}", dir.display(), e);
                return loaded;
            }
        };
        for entry in entries.flatten() {
            let path = entry.path();
            let is_plugin = path.extension()
                .map(|e| e == "so" || e == "dylib")
                .unwrap_or(false);
            if is_plugin {
                match self.load(&path) {
                    Ok(name) => loaded.push(name),
                    Err(e) => tracing::warn!("Plugin load failed '{}': {}", path.display(), e),
                }
            }
        }
        loaded
    }
}
