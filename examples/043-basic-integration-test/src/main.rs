// ╔════════════════════════════════════════════════════════════════════════╗
// ║  043 — Task Manager Integration Tests                               ║
// ╠════════════════════════════════════════════════════════════════════════╣
// ║  Domain:   Developer Experience — Testing Best Practices            ║
// ║  Pattern:  VX_APP                                                    ║
// ║  Features: ServiceProcess, ServiceCtx, ShmSlice, VilResponse,       ║
// ║            VilModel, TestClient (vil_server_test)                    ║
// ╠════════════════════════════════════════════════════════════════════════╣
// ║  Business: A simple task CRUD API demonstrating how to write         ║
// ║  integration tests for VIL applications using TestClient.            ║
// ║  TestClient dispatches requests directly to the Axum router          ║
// ║  without network overhead — fast, deterministic, no port binding.    ║
// ║                                                                      ║
// ║  Endpoints:                                                          ║
// ║    POST /api/tasks       → create a task                             ║
// ║    GET  /api/tasks       → list all tasks                            ║
// ║    GET  /api/tasks/stats → task statistics                           ║
// ║                                                                      ║
// ║  The #[cfg(test)] module below shows the VIL Way to test:            ║
// ║    1. Build a Router from ServiceProcess.build_router()              ║
// ║    2. Wrap with TestClient                                           ║
// ║    3. Hit endpoints, assert status codes and JSON bodies             ║
// ╚════════════════════════════════════════════════════════════════════════╝
//
// Run:   cargo run -p vil-basic-integration-test
// Test:  cargo test -p vil-basic-integration-test

use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use tokio::sync::RwLock;
use vil_server::prelude::*;

// ── Models (VilModel = SIMD-ready serialization) ─────────────────────────

#[derive(Clone, Debug, Serialize, Deserialize, VilModel)]
struct Task {
    id: u64,
    title: String,
    done: bool,
}

#[derive(Debug, Deserialize)]
struct CreateTaskRequest {
    title: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, VilModel)]
struct TaskListResponse {
    tasks: Vec<Task>,
    count: usize,
}

#[derive(Clone, Debug, Serialize, Deserialize, VilModel)]
struct TaskStatsResponse {
    total: u64,
    done: u64,
    pending: u64,
}

// ── Shared State (via ServiceCtx, not Extension<T>) ──────────────────────

struct TaskStore {
    tasks: RwLock<Vec<Task>>,
    next_id: AtomicU64,
}

// ── Handlers ─────────────────────────────────────────────────────────────

/// POST /tasks — Create a new task.
/// ShmSlice: zero-copy body from ExchangeHeap (not Json<T>).
/// ServiceCtx: state access via ctx.state (not Extension<T>).
async fn create_task(
    ctx: ServiceCtx,
    body: ShmSlice,
) -> HandlerResult<VilResponse<Task>> {
    let req: CreateTaskRequest = body.json()
        .map_err(|_| VilError::bad_request("invalid JSON — expected {title}"))?;

    if req.title.trim().is_empty() {
        return Err(VilError::bad_request("title must not be empty"));
    }

    let store = ctx.state::<Arc<TaskStore>>()
        .map_err(|_| VilError::internal("state not found"))?;

    let id = store.next_id.fetch_add(1, Ordering::Relaxed) + 1;
    let task = Task {
        id,
        title: req.title,
        done: false,
    };

    store.tasks.write().await.push(task.clone());
    Ok(VilResponse::ok(task))
}

/// GET /tasks — List all tasks.
async fn list_tasks(ctx: ServiceCtx) -> HandlerResult<VilResponse<TaskListResponse>> {
    let store = ctx.state::<Arc<TaskStore>>()
        .map_err(|_| VilError::internal("state not found"))?;

    let tasks = store.tasks.read().await.clone();
    let count = tasks.len();
    Ok(VilResponse::ok(TaskListResponse { tasks, count }))
}

/// GET /tasks/stats — Task statistics.
async fn task_stats(ctx: ServiceCtx) -> HandlerResult<VilResponse<TaskStatsResponse>> {
    let store = ctx.state::<Arc<TaskStore>>()
        .map_err(|_| VilError::internal("state not found"))?;

    let tasks = store.tasks.read().await;
    let total = tasks.len() as u64;
    let done = tasks.iter().filter(|t| t.done).count() as u64;
    Ok(VilResponse::ok(TaskStatsResponse {
        total,
        done,
        pending: total - done,
    }))
}

// ── Main ─────────────────────────────────────────────────────────────────

#[tokio::main]
async fn main() {
    let store = Arc::new(TaskStore {
        tasks: RwLock::new(Vec::new()),
        next_id: AtomicU64::new(0),
    });

    let svc = ServiceProcess::new("tasks")
        .endpoint(Method::POST, "/tasks", post(create_task))
        .endpoint(Method::GET, "/tasks", get(list_tasks))
        .endpoint(Method::GET, "/tasks/stats", get(task_stats))
        .state(store);

    VilApp::new("task-manager")
        .port(8080)
        .service(svc)
        .run()
        .await;
}

// ── Integration Tests ───────────────────────────────────────────────────
//
// Uses vil_server_test::TestClient for direct router dispatch.
// No network, no port binding, no race conditions.

#[cfg(test)]
mod tests {
    use super::*;
    use vil_server::axum;
    use vil_server_core::AppState;
    use vil_server_test::TestClient;

    /// Build a fresh test app with an empty TaskStore.
    /// Each test gets its own isolated state — no cross-test contamination.
    fn build_test_app() -> axum::Router {
        let store = Arc::new(TaskStore {
            tasks: RwLock::new(Vec::new()),
            next_id: AtomicU64::new(0),
        });

        let svc = ServiceProcess::new("tasks")
            .endpoint(Method::POST, "/tasks", post(create_task))
            .endpoint(Method::GET, "/tasks", get(list_tasks))
            .endpoint(Method::GET, "/tasks/stats", get(task_stats))
            .state(store);

        let state = AppState::new("test-task-manager");
        svc.build_router().with_state(state)
    }

    #[tokio::test]
    async fn test_list_empty() {
        let client = TestClient::new(build_test_app());

        let resp = client.get("/api/tasks").await;
        resp.assert_ok();

        let body: serde_json::Value = resp.json();
        assert_eq!(body["data"]["count"], 0);
        assert!(body["data"]["tasks"].as_array().unwrap().is_empty());
    }

    #[tokio::test]
    async fn test_create_task() {
        let client = TestClient::new(build_test_app());

        let resp = client.post_json(
            "/api/tasks",
            r#"{"title":"Write integration tests"}"#,
        ).await;
        resp.assert_ok();

        let body: serde_json::Value = resp.json();
        assert_eq!(body["data"]["id"], 1);
        assert_eq!(body["data"]["title"], "Write integration tests");
        assert_eq!(body["data"]["done"], false);
    }

    #[tokio::test]
    async fn test_create_and_list() {
        let client = TestClient::new(build_test_app());

        // Create two tasks
        client.post_json("/api/tasks", r#"{"title":"Task A"}"#).await.assert_ok();
        client.post_json("/api/tasks", r#"{"title":"Task B"}"#).await.assert_ok();

        // List should show both
        let resp = client.get("/api/tasks").await;
        resp.assert_ok();

        let body: serde_json::Value = resp.json();
        assert_eq!(body["data"]["count"], 2);
        let tasks = body["data"]["tasks"].as_array().unwrap();
        assert_eq!(tasks[0]["title"], "Task A");
        assert_eq!(tasks[1]["title"], "Task B");
    }

    #[tokio::test]
    async fn test_stats_after_create() {
        let client = TestClient::new(build_test_app());

        // Empty stats
        let resp = client.get("/api/tasks/stats").await;
        resp.assert_ok();
        let body: serde_json::Value = resp.json();
        assert_eq!(body["data"]["total"], 0);
        assert_eq!(body["data"]["pending"], 0);

        // Create a task then check stats
        client.post_json("/api/tasks", r#"{"title":"Task 1"}"#).await.assert_ok();
        let resp = client.get("/api/tasks/stats").await;
        resp.assert_ok();
        let body: serde_json::Value = resp.json();
        assert_eq!(body["data"]["total"], 1);
        assert_eq!(body["data"]["pending"], 1);
        assert_eq!(body["data"]["done"], 0);
    }

    #[tokio::test]
    async fn test_create_empty_title_rejected() {
        let client = TestClient::new(build_test_app());

        let resp = client.post_json("/api/tasks", r#"{"title":""}"#).await;
        // Should return 400 Bad Request
        resp.assert_status(StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn test_create_invalid_json_rejected() {
        let client = TestClient::new(build_test_app());

        let resp = client.post_json("/api/tasks", "not valid json").await;
        resp.assert_status(StatusCode::BAD_REQUEST);
    }
}
