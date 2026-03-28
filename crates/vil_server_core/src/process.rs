// =============================================================================
// VIL Server Process — Process-per-handler isolation
// =============================================================================
//
// Each route handler is registered as an isolated VIL process in the runtime.
// This provides:
//   - Fault isolation: crash in one handler doesn't affect others
//   - Per-handler SHM regions: memory usage tracking per route
//   - Per-handler metrics: publish/recv/drop counters via RuntimeCounters
//   - Clean shutdown: supervisor can drain individual handlers

use dashmap::DashMap;
use std::sync::Arc;
use vil_log::app_log;

use vil_rt::{ProcessHandle, VastarRuntimeWorld};
use vil_types::{
    CleanupPolicy, ExecClass, PortDirection, PortSpec, ProcessSpec,
};

/// Registry of VIL processes corresponding to HTTP handlers.
///
/// Each handler route gets its own VIL process, providing:
/// - Crash isolation (one handler failure doesn't affect others)
/// - Per-handler metrics and SHM accounting
/// - Clean supervisor lifecycle management
pub struct ProcessRegistry {
    runtime: Arc<VastarRuntimeWorld>,
    /// Map of "METHOD /path" -> ProcessHandle
    handles: DashMap<String, ProcessHandle>,
}

impl ProcessRegistry {
    pub fn new(runtime: Arc<VastarRuntimeWorld>) -> Self {
        Self {
            runtime,
            handles: DashMap::new(),
        }
    }

    /// Register a handler route as a VIL process.
    ///
    /// The process is created with:
    /// - ExecClass::Async (server handlers are async by default)
    /// - CleanupPolicy::RecoverAndRestart (auto-recover on crash)
    /// - Two ports: "http_in" (In) and "http_out" (Out)
    pub fn register_handler(&self, method: &str, path: &str) -> Option<ProcessHandle> {
        let key = format!("{} {}", method, path);

        // Don't register duplicates
        if self.handles.contains_key(&key) {
            return self.handles.get(&key).map(|h| h.value().clone());
        }

        // Leak strings for 'static lifetime (these live for program duration)
        let id_str: &'static str = Box::leak(key.clone().into_boxed_str());
        let name_str: &'static str = Box::leak(format!("handler:{}", key).into_boxed_str());

        let ports: &'static [PortSpec] = Box::leak(vec![
            PortSpec {
                name: "http_in",
                direction: PortDirection::In,
                ..PortSpec::default()
            },
            PortSpec {
                name: "http_out",
                direction: PortDirection::Out,
                ..PortSpec::default()
            },
        ].into_boxed_slice());

        let spec = ProcessSpec {
            id: id_str,
            name: name_str,
            exec: ExecClass::AsyncTask,
            cleanup: CleanupPolicy::ReclaimOrphans,
            ports,
            observability: Default::default(),
        };

        match self.runtime.register_process(spec) {
            Ok(handle) => {
                app_log!(Info, "handler.process.registered", { handler: key.as_str() });
                self.handles.insert(key, handle.clone());
                Some(handle)
            }
            Err(e) => {
                app_log!(Error, "handler.process.failed", { handler: key.as_str(), error: format!("{:?}", e) });
                None
            }
        }
    }

    /// Get the ProcessHandle for a handler route.
    pub fn get_handle(&self, method: &str, path: &str) -> Option<ProcessHandle> {
        let key = format!("{} {}", method, path);
        self.handles.get(&key).map(|h| h.value().clone())
    }

    /// Get the number of registered handler processes.
    pub fn handler_count(&self) -> usize {
        self.handles.len()
    }

    /// List all registered handler process keys.
    pub fn handler_keys(&self) -> Vec<String> {
        self.handles.iter().map(|e| e.key().clone()).collect()
    }

    /// Get the runtime reference.
    pub fn runtime(&self) -> &Arc<VastarRuntimeWorld> {
        &self.runtime
    }
}
