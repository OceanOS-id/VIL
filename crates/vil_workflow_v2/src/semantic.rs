//! Semantic types for workflow orchestration operations.
//!
//! These types follow VIL's process-oriented semantic model:
//! - Events: immutable audit records (Data Lane)
//! - Faults: error signals (Control Lane)
//! - State: mutable tracked state (Data Lane)

use serde::{Deserialize, Serialize};
use vil_macros::{VilAiEvent, VilAiFault, VilAiState};

// ── Events (Data Lane, immutable audit) ─────────────────────────────

/// Emitted after a workflow execution completes.
#[derive(Clone, Debug, Serialize, Deserialize, VilAiEvent)]
pub struct WorkflowEvent {
    pub task_count: usize,
    pub completed: usize,
    pub failed: usize,
    pub total_ms: u64,
    pub parallelism_ratio: f64,
}

// ── Faults (Control Lane, error signals) ────────────────────────────

/// Classification of workflow failure modes.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum WorkflowFaultType {
    CycleDetected,
    MissingDependency,
    TaskTimeout,
    TaskFailed,
    SchedulerError,
}

/// Emitted when a workflow operation fails.
#[derive(Clone, Debug, Serialize, Deserialize, VilAiFault)]
pub struct WorkflowFault {
    pub error_type: WorkflowFaultType,
    pub message: String,
    pub task_id: Option<String>,
}

impl WorkflowFault {
    pub fn cycle_detected(msg: &str) -> Self {
        Self {
            error_type: WorkflowFaultType::CycleDetected,
            message: msg.into(),
            task_id: None,
        }
    }

    pub fn task_timeout(task_id: &str) -> Self {
        Self {
            error_type: WorkflowFaultType::TaskTimeout,
            message: format!("task {} timed out", task_id),
            task_id: Some(task_id.into()),
        }
    }
}

// ── State (Data Lane, mutable tracked) ──────────────────────────────

/// Tracks cumulative workflow execution statistics.
#[derive(Clone, Debug, Default, Serialize, Deserialize, VilAiState)]
pub struct WorkflowState {
    pub total_workflows: u64,
    pub total_tasks_executed: u64,
    pub total_failures: u64,
    pub total_timeouts: u64,
    pub avg_parallelism_ratio: f64,
}

impl WorkflowState {
    pub fn record(&mut self, event: &WorkflowEvent) {
        self.total_workflows += 1;
        self.total_tasks_executed += event.completed as u64;
        self.total_failures += event.failed as u64;
        let n = self.total_workflows as f64;
        self.avg_parallelism_ratio =
            self.avg_parallelism_ratio * (n - 1.0) / n + event.parallelism_ratio / n;
    }

    pub fn record_error(&mut self) {
        self.total_failures += 1;
    }
}
