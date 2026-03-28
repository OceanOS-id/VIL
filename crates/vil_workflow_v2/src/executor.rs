use std::time::Instant;

use crate::task::{Task, TaskResult, TaskStatus, TaskType};

/// Execute a single task (simulated). In production, this would dispatch
/// to actual AI services.
pub async fn execute_task(task: &Task) -> TaskResult {
    let start = Instant::now();

    // Simulate work based on task type
    let (output, status) = if task.timeout_ms > 0 && task.timeout_ms < 5 {
        // Simulate timeout for very small timeout values (used in tests)
        (serde_json::json!({"error": "timeout"}), TaskStatus::Timeout)
    } else {
        let output = match &task.task_type {
            TaskType::Embed => serde_json::json!({"embedding": [0.1, 0.2, 0.3]}),
            TaskType::Search => serde_json::json!({"results": ["doc1", "doc2"]}),
            TaskType::Generate => serde_json::json!({"text": "generated output"}),
            TaskType::Transform => serde_json::json!({"transformed": true}),
            TaskType::Rerank => serde_json::json!({"reranked": ["doc2", "doc1"]}),
            TaskType::Filter => serde_json::json!({"filtered": true, "remaining": 5}),
            TaskType::Branch => serde_json::json!({"branch": "taken"}),
            TaskType::Merge => serde_json::json!({"merged": true}),
            TaskType::Cache => serde_json::json!({"cached": true}),
            TaskType::Custom(name) => serde_json::json!({"custom": name}),
        };
        (output, TaskStatus::Success)
    };

    let duration_ms = start.elapsed().as_millis() as u64;

    TaskResult {
        task_id: task.id.clone(),
        output,
        duration_ms,
        status,
    }
}
