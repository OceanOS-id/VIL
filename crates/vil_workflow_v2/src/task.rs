use serde::{Deserialize, Serialize};

/// Type of task in a workflow.
/// Aligned with vil_ai_compiler::PipelineNode variants.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TaskType {
    Embed,
    Search,
    Generate,
    Transform,
    Rerank,
    Filter,
    Branch,
    Merge,
    Cache,
    Custom(String),
}

/// Status of a completed task.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TaskStatus {
    Success,
    Failed(String),
    Timeout,
    Skipped,
}

/// A single task in a workflow DAG.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    pub id: String,
    pub name: String,
    pub task_type: TaskType,
    /// IDs of tasks that must complete before this one.
    pub deps: Vec<String>,
    /// Maximum execution time in milliseconds (0 = no timeout).
    pub timeout_ms: u64,
    /// Arbitrary configuration from YAML (model params, index name, etc.)
    #[serde(default)]
    pub config: Option<serde_json::Value>,
    /// Which topology node this task runs inside (Layer 1 ↔ Layer 2 binding).
    #[serde(default)]
    pub node_binding: Option<String>,
}

impl Task {
    pub fn new(id: impl Into<String>, name: impl Into<String>, task_type: TaskType) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            task_type,
            deps: Vec::new(),
            timeout_ms: 0,
            config: None,
            node_binding: None,
        }
    }

    pub fn with_deps(mut self, deps: Vec<String>) -> Self {
        self.deps = deps;
        self
    }

    pub fn with_timeout(mut self, ms: u64) -> Self {
        self.timeout_ms = ms;
        self
    }

    pub fn with_config(mut self, config: serde_json::Value) -> Self {
        self.config = Some(config);
        self
    }

    pub fn with_node_binding(mut self, node: impl Into<String>) -> Self {
        self.node_binding = Some(node.into());
        self
    }
}

/// Result of executing a single task.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskResult {
    pub task_id: String,
    pub output: serde_json::Value,
    pub duration_ms: u64,
    pub status: TaskStatus,
}
