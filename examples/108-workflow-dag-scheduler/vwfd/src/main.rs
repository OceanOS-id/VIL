// 108 — ETL Workflow DAG Scheduler (VWFD)
// Business logic identical to standard:
//   DAG: extract → [validate, transform] (parallel) → load → notify
//   5 tasks with dependency resolution and simulated execution
use serde_json::{json, Value};

fn run_workflow(_input: &Value) -> Result<Value, String> {
    let start = std::time::Instant::now();

    let tasks = vec![
        json!({"id": "extract", "desc": "Extract from API", "status": "completed", "duration_ms": 2}),
        json!({"id": "validate", "desc": "Validate schema", "status": "completed", "duration_ms": 1, "depends_on": ["extract"]}),
        json!({"id": "transform", "desc": "Transform records", "status": "completed", "duration_ms": 1, "depends_on": ["extract"]}),
        json!({"id": "load", "desc": "Load to warehouse", "status": "completed", "duration_ms": 3, "depends_on": ["validate", "transform"]}),
        json!({"id": "notify", "desc": "Send notification", "status": "completed", "duration_ms": 1, "depends_on": ["load"]}),
    ];

    let total_ms = start.elapsed().as_millis() as u64;

    Ok(json!({
        "workflow_id": "etl-daily",
        "status": "completed",
        "tasks": tasks,
        "total_tasks": 5,
        "completed_tasks": 5,
        "total_duration_ms": total_ms,
        "parallel_branches": ["validate", "transform"]
    }))
}

fn workflow_status(_input: &Value) -> Result<Value, String> {
    Ok(json!({
        "workflow_id": "etl-daily",
        "status": "idle",
        "last_run": "completed",
        "total_runs": 0,
        "scheduler": "DAG-based with parallel branch resolution"
    }))
}

#[tokio::main]
async fn main() {
    vil_vwfd::app("examples/108-workflow-dag-scheduler/vwfd/workflows", 8080)
        .native("run_workflow", run_workflow)
        .native("workflow_status", workflow_status)
        .run().await;
}
