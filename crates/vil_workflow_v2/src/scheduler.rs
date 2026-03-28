use std::collections::HashMap;
use std::time::Instant;

use serde::{Deserialize, Serialize};

use crate::dag;
use crate::executor;
use crate::task::{Task, TaskResult};

/// Result of an entire workflow execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowResult {
    pub results: Vec<TaskResult>,
    pub total_ms: u64,
    /// Ratio of parallelism achieved (1.0 = fully sequential, > 1.0 = parallel speedup).
    pub parallelism_ratio: f64,
}

/// DAG-based workflow scheduler that resolves dependencies and executes
/// tasks in parallel where possible.
#[derive(Debug, Default)]
pub struct WorkflowScheduler;

impl WorkflowScheduler {
    pub fn new() -> Self {
        Self
    }

    /// Submit a set of tasks for execution. Tasks are scheduled according
    /// to their dependency graph and executed in parallel within each layer.
    pub async fn submit(&self, tasks: Vec<Task>) -> Result<WorkflowResult, String> {
        if tasks.is_empty() {
            return Ok(WorkflowResult {
                results: Vec::new(),
                total_ms: 0,
                parallelism_ratio: 1.0,
            });
        }

        let start = Instant::now();
        let task_map: HashMap<String, Task> =
            tasks.iter().map(|t| (t.id.clone(), t.clone())).collect();

        let layers = dag::resolve_layers(&tasks)?;
        let num_layers = layers.len();
        let num_tasks = tasks.len();

        let mut all_results = Vec::new();

        for layer in &layers {
            let mut handles = Vec::new();
            for task_id in layer {
                let task = task_map[task_id].clone();
                handles.push(tokio::spawn(
                    async move { executor::execute_task(&task).await },
                ));
            }

            for handle in handles {
                match handle.await {
                    Ok(result) => all_results.push(result),
                    Err(e) => {
                        return Err(format!("Task execution failed: {}", e));
                    }
                }
            }
        }

        let total_ms = start.elapsed().as_millis() as u64;
        let parallelism_ratio = if num_layers > 0 {
            num_tasks as f64 / num_layers as f64
        } else {
            1.0
        };

        Ok(WorkflowResult {
            results: all_results,
            total_ms,
            parallelism_ratio,
        })
    }
}
