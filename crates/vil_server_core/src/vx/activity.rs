// =============================================================================
// VX Activity Chain — Multi-step business logic within a single endpoint
// =============================================================================
//
// An Activity is a single unit of business logic execution within a VIL Process.
// Multiple activities can be chained within one endpoint — data stays in SHM,
// only the active handler and token change between activities (Tri-Lane).
//
// Activity types:
//   - Native:   Rust async fn — compiled, zero overhead
//   - Wasm:     WASM module function — sandboxed, hot-deployable
//   - Sidecar:  External process (Python/Go/Java) — polyglot, ML models
//
// Developer writes business logic + declares chain topology.
// VIL generates all plumbing (SHM token, dispatch, error handling, tracing).
//
// # Example
//
// ```ignore
// use vil_server::prelude::*;
//
// async fn validate(body: ShmSlice) -> ActivityResult {
//     let order: Order = body.json()?;
//     if order.qty == 0 { return Err(ActivityError::validation("qty must be > 0")); }
//     ActivityResult::ok(serde_json::to_vec(&order)?)
// }
//
// let orders = ServiceProcess::new("orders")
//     .chain_endpoint(Method::POST, "/order", "process_order",
//         ActivityChain::new()
//             .native("validate", validate)
//             .wasm("price", "pricing", "calculate_price")
//             .sidecar("fraud", "fraud-checker", "check_fraud")
//             .native("finalize", finalize_order)
//     );
// ```

use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use serde::{Deserialize, Serialize};

/// Result of a single activity execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActivityOutput {
    /// Output data bytes (written back to SHM region)
    pub data: Vec<u8>,
    /// Activity-specific metadata for tracing
    pub metadata: Option<serde_json::Value>,
}

/// Error from activity execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActivityError {
    pub activity_name: String,
    pub kind: ActivityErrorKind,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ActivityErrorKind {
    Validation,
    WasmExecution,
    SidecarTimeout,
    SidecarError,
    Internal,
}

impl ActivityError {
    pub fn validation(msg: impl Into<String>) -> Self {
        Self {
            activity_name: String::new(),
            kind: ActivityErrorKind::Validation,
            message: msg.into(),
        }
    }

    pub fn internal(msg: impl Into<String>) -> Self {
        Self {
            activity_name: String::new(),
            kind: ActivityErrorKind::Internal,
            message: msg.into(),
        }
    }

    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.activity_name = name.into();
        self
    }
}

impl std::fmt::Display for ActivityError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[{}] {:?}: {}", self.activity_name, self.kind, self.message)
    }
}

impl std::error::Error for ActivityError {}

/// Boxed future for native activity functions.
pub type ActivityFuture =
    Pin<Box<dyn Future<Output = Result<ActivityOutput, ActivityError>> + Send>>;

/// A native (Rust) activity function.
///
/// Takes input bytes (from SHM) and returns output bytes (written back to SHM).
/// Developer writes only business logic — VIL handles SHM token management.
pub type NativeActivityFn = Arc<dyn Fn(Vec<u8>) -> ActivityFuture + Send + Sync>;

/// Describes one step in an activity chain.
#[derive(Clone)]
pub struct ActivityStep {
    /// Human-readable name (for tracing and error reporting)
    pub name: String,
    /// Execution mode
    pub mode: ActivityMode,
}

/// How this activity is executed.
#[derive(Clone)]
pub enum ActivityMode {
    /// Compiled Rust async function — zero overhead.
    Native {
        handler: NativeActivityFn,
    },
    /// WASM module function — sandboxed, hot-deployable.
    /// VIL runtime calls WasmPool::call_i32() or call_with_memory().
    Wasm {
        module_name: String,
        function_name: String,
    },
    /// External sidecar process — polyglot (Python/Go/Java).
    /// VIL runtime calls dispatcher::invoke() via SHM+UDS.
    Sidecar {
        target_name: String,
        method_name: String,
    },
}

/// A chain of activities executed sequentially within one endpoint.
///
/// Data flows through SHM — each activity reads input from SHM, writes output
/// back to SHM. The Tri-Lane token transfers between activities automatically.
///
/// Developer builds the chain declaratively. VIL generates all plumbing.
pub struct ActivityChain {
    steps: Vec<ActivityStep>,
}

impl ActivityChain {
    /// Create an empty chain.
    pub fn new() -> Self {
        Self { steps: Vec::new() }
    }

    /// Add a native Rust activity to the chain.
    ///
    /// The function receives input bytes and returns output bytes.
    /// VIL handles SHM read/write and token management.
    pub fn native<F, Fut>(mut self, name: impl Into<String>, handler: F) -> Self
    where
        F: Fn(Vec<u8>) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<ActivityOutput, ActivityError>> + Send + 'static,
    {
        let handler = Arc::new(move |data: Vec<u8>| -> ActivityFuture {
            Box::pin(handler(data))
        });
        self.steps.push(ActivityStep {
            name: name.into(),
            mode: ActivityMode::Native { handler },
        });
        self
    }

    /// Add a WASM activity to the chain.
    ///
    /// VIL runtime resolves the module from WasmFaaSRegistry and calls
    /// the specified function. Developer does NOT call pool.call() manually.
    pub fn wasm(
        mut self,
        name: impl Into<String>,
        module: impl Into<String>,
        function: impl Into<String>,
    ) -> Self {
        self.steps.push(ActivityStep {
            name: name.into(),
            mode: ActivityMode::Wasm {
                module_name: module.into(),
                function_name: function.into(),
            },
        });
        self
    }

    /// Add a sidecar activity to the chain.
    ///
    /// VIL runtime resolves the target from SidecarRegistry and calls
    /// dispatcher::invoke(). Developer does NOT call dispatcher manually.
    pub fn sidecar(
        mut self,
        name: impl Into<String>,
        target: impl Into<String>,
        method: impl Into<String>,
    ) -> Self {
        self.steps.push(ActivityStep {
            name: name.into(),
            mode: ActivityMode::Sidecar {
                target_name: target.into(),
                method_name: method.into(),
            },
        });
        self
    }

    /// Get the steps in this chain.
    pub fn steps(&self) -> &[ActivityStep] {
        &self.steps
    }

    /// Number of steps in the chain.
    pub fn len(&self) -> usize {
        self.steps.len()
    }

    /// Whether the chain is empty.
    pub fn is_empty(&self) -> bool {
        self.steps.is_empty()
    }
}

impl Default for ActivityChain {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Debug for ActivityChain {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ActivityChain")
            .field("steps", &self.steps.iter().map(|s| {
                match &s.mode {
                    ActivityMode::Native { .. } => format!("{}(native)", s.name),
                    ActivityMode::Wasm { module_name, function_name } =>
                        format!("{}(wasm:{}::{})", s.name, module_name, function_name),
                    ActivityMode::Sidecar { target_name, method_name } =>
                        format!("{}(sidecar:{}::{})", s.name, target_name, method_name),
                }
            }).collect::<Vec<_>>())
            .finish()
    }
}

/// Execution trace for a completed chain — included in response for observability.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChainTrace {
    pub steps: Vec<StepTrace>,
    pub total_ms: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StepTrace {
    pub name: String,
    pub mode: String,
    pub elapsed_ms: f64,
    pub status: String,
}
