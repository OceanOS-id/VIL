// =============================================================================
// VX Endpoint — HTTP endpoint definition for Process-oriented routing
// =============================================================================
//
// Each EndpointDef describes a single HTTP endpoint that maps to a
// ServiceProcess handler. In Phase 1 the handler is an Axum MethodRouter;
// in Phase 2 it will be a raw SHM descriptor consumer.

use axum::http::Method;
use axum::routing::MethodRouter;

use crate::state::AppState;

/// Execution class for an endpoint handler.
///
/// Determines how the VIL runtime schedules the handler.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExecClass {
    /// Runs on the Tokio async executor (default, best for I/O-bound).
    AsyncTask,
    /// Runs on `spawn_blocking` (for CPU-bound or legacy sync code).
    BlockingTask,
    /// Runs on a dedicated OS thread (for FFI, GPU, or pinned workloads).
    DedicatedThread,
    /// Pinned to a specific worker thread (for latency-critical paths).
    PinnedWorker,
    /// Runs inside a WASM capsule (sandboxed, hot-reloadable).
    WasmCapsule,
    /// Routes to an external sidecar process via SHM IPC.
    SidecarProcess,
    /// Enhanced WASM FaaS with pooling, memory limits, and timeout.
    WasmFaaS,
}

impl Default for ExecClass {
    fn default() -> Self {
        ExecClass::AsyncTask
    }
}

impl std::fmt::Display for ExecClass {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ExecClass::AsyncTask => write!(f, "AsyncTask"),
            ExecClass::BlockingTask => write!(f, "BlockingTask"),
            ExecClass::DedicatedThread => write!(f, "DedicatedThread"),
            ExecClass::PinnedWorker => write!(f, "PinnedWorker"),
            ExecClass::WasmCapsule => write!(f, "WasmCapsule"),
            ExecClass::SidecarProcess => write!(f, "SidecarProcess"),
            ExecClass::WasmFaaS => write!(f, "WasmFaaS"),
        }
    }
}

/// Describes one HTTP endpoint that maps to a ServiceProcess handler.
pub struct EndpointDef {
    /// HTTP method (GET, POST, etc.)
    pub method: Method,
    /// URL path (relative to service prefix, e.g. "/users/:id")
    pub path: String,
    /// Human-readable handler name (for observability and topology export)
    pub handler_name: String,
    /// Axum method router (Phase 1 bridge)
    pub handler: Option<MethodRouter<AppState>>,
    /// Execution class override for this endpoint
    pub exec_class: ExecClass,
    /// Target sidecar name (only used when exec_class = SidecarProcess).
    pub sidecar_target: Option<String>,
    /// WASM module name (only used when exec_class = WasmFaaS).
    pub wasm_module: Option<String>,
}

impl EndpointDef {
    /// Create a new endpoint definition.
    pub fn new(method: Method, path: impl Into<String>, handler_name: impl Into<String>) -> Self {
        Self {
            method,
            path: path.into(),
            handler_name: handler_name.into(),
            handler: None,
            exec_class: ExecClass::default(),
            sidecar_target: None,
            wasm_module: None,
        }
    }

    /// Attach an Axum MethodRouter handler (Phase 1).
    pub fn handler(mut self, handler: MethodRouter<AppState>) -> Self {
        self.handler = Some(handler);
        self
    }

    /// Override the execution class for this endpoint.
    pub fn exec(mut self, exec_class: ExecClass) -> Self {
        self.exec_class = exec_class;
        self
    }

    /// Set the sidecar target name (for ExecClass::SidecarProcess).
    pub fn sidecar_target(mut self, target: impl Into<String>) -> Self {
        self.sidecar_target = Some(target.into());
        self
    }

    /// Set the WASM module name (for ExecClass::WasmFaaS).
    pub fn wasm_module(mut self, module: impl Into<String>) -> Self {
        self.wasm_module = Some(module.into());
        self
    }
}

impl std::fmt::Debug for EndpointDef {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EndpointDef")
            .field("method", &self.method.as_str())
            .field("path", &self.path)
            .field("handler_name", &self.handler_name)
            .field("exec_class", &self.exec_class)
            .field("sidecar_target", &self.sidecar_target)
            .field("wasm_module", &self.wasm_module)
            .field("has_handler", &self.handler.is_some())
            .finish()
    }
}
