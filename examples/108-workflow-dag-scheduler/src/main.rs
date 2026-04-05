// ╔════════════════════════════════════════════════════════════╗
// ║  108 — ETL Workflow DAG Scheduler                         ║
// ╠════════════════════════════════════════════════════════════╣
// ║  Domain:   Data Engineering — ETL Orchestration            ║
// ║  Pattern:  VX_APP                                           ║
// ║  Features: WorkflowScheduler, Task, TaskType, DAG deps,   ║
// ║            parallel execution, timeout handling             ║
// ╠════════════════════════════════════════════════════════════╣
// ║  Business: DAG with 5 tasks:                                ║
// ║    extract → [validate, transform] (parallel) → load → notify║
// ║  WorkflowScheduler resolves dependencies, runs parallel    ║
// ║  branches, enforces timeouts per task.                      ║
// ╚════════════════════════════════════════════════════════════╝
//
// Run:   cargo run -p vil-workflow-dag-scheduler
// Test:
//   curl -X POST http://localhost:8080/api/workflow/run
//   curl http://localhost:8080/api/workflow/status

use std::sync::{Arc, Mutex};

use vil_server::prelude::*;
use vil_workflow_v2::{Task, TaskType, WorkflowScheduler};

#[derive(Clone, Debug, Serialize, Deserialize, VilModel)]
struct WorkflowRunResponse {
    status: String,
    tasks_total: usize,
    tasks_completed: usize,
    tasks_failed: usize,
    duration_ms: f64,
    task_results: Vec<TaskResultInfo>,
}

#[derive(Clone, Debug, Serialize, Deserialize, VilModel)]
struct TaskResultInfo {
    id: String,
    status: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, VilModel)]
struct WorkflowStatusResponse {
    last_run: Option<WorkflowRunResponse>,
    total_runs: u64,
}

struct AppState {
    last_run: Mutex<Option<WorkflowRunResponse>>,
    total_runs: Mutex<u64>,
}

async fn run_workflow(ctx: ServiceCtx) -> HandlerResult<VilResponse<WorkflowRunResponse>> {
    let start = std::time::Instant::now();

    let tasks = vec![
        Task::new("extract", "Extract from API", TaskType::Custom("extract".into()))
            .with_timeout(10000),
        Task::new("validate", "Validate schema", TaskType::Filter)
            .with_deps(vec!["extract".into()])
            .with_timeout(5000),
        Task::new("transform", "Transform + enrich", TaskType::Transform)
            .with_deps(vec!["extract".into()])
            .with_timeout(5000),
        Task::new("load", "Load to database", TaskType::Custom("load".into()))
            .with_deps(vec!["validate".into(), "transform".into()])
            .with_timeout(15000),
        Task::new("notify", "Send notification", TaskType::Custom("notify".into()))
            .with_deps(vec!["load".into()])
            .with_timeout(5000),
    ];

    let task_count = tasks.len();
    let scheduler = WorkflowScheduler::new();
    let result = scheduler.submit(tasks).await;

    let duration_ms = start.elapsed().as_secs_f64() * 1000.0;

    let response = match result {
        Ok(wf_result) => {
            let task_results: Vec<TaskResultInfo> = wf_result.results.iter()
                .map(|tr| TaskResultInfo {
                    id: tr.task_id.clone(),
                    status: format!("{:?}", tr.status),
                })
                .collect();
            let failed = task_results.iter().filter(|t| t.status.contains("Failed")).count();
            WorkflowRunResponse {
                status: if failed == 0 { "completed".into() } else { "partial_failure".into() },
                tasks_total: task_count,
                tasks_completed: task_results.len() - failed,
                tasks_failed: failed,
                duration_ms,
                task_results,
            }
        }
        Err(e) => WorkflowRunResponse {
            status: format!("error: {}", e),
            tasks_total: task_count, tasks_completed: 0, tasks_failed: task_count,
            duration_ms, task_results: vec![],
        },
    };

    let state = ctx.state::<Arc<AppState>>().map_err(|_| VilError::internal("state"))?;
    *state.last_run.lock().unwrap() = Some(response.clone());
    *state.total_runs.lock().unwrap() += 1;

    Ok(VilResponse::ok(response))
}

async fn workflow_status(ctx: ServiceCtx) -> HandlerResult<VilResponse<WorkflowStatusResponse>> {
    let state = ctx.state::<Arc<AppState>>().map_err(|_| VilError::internal("state"))?;
    Ok(VilResponse::ok(WorkflowStatusResponse {
        last_run: state.last_run.lock().unwrap().clone(),
        total_runs: *state.total_runs.lock().unwrap(),
    }))
}

#[tokio::main]
async fn main() {
    let state = Arc::new(AppState {
        last_run: Mutex::new(None),
        total_runs: Mutex::new(0),
    });

    let svc = ServiceProcess::new("workflow")
        .endpoint(Method::POST, "/run", post(run_workflow))
        .endpoint(Method::GET, "/status", get(workflow_status))
        .state(state);

    VilApp::new("etl-dag-scheduler")
        .port(8080)
        .observer(true)
        .service(svc)
        .run()
        .await;
}
