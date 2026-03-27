// ╔════════════════════════════════════════════════════════════╗
// ║  004 — Task Management System (Project Management Tool)   ║
// ╠════════════════════════════════════════════════════════════╣
// ║  Domain:   Project Management — Sprint/Task Tracking       ║
// ║  Pattern:  VX_APP                                           ║
// ║  Token:    N/A (HTTP server)                                ║
// ║  Features: ShmSlice, ServiceCtx, VilResponse                ║
// ╠════════════════════════════════════════════════════════════╣
// ║  Business: REST API for engineering teams to manage tasks   ║
// ║  within sprints. Supports create, read, update, delete     ║
// ║  with input validation. In-memory store for demo; swap     ║
// ║  with PostgreSQL (via vlang_db_sqlx) for production.       ║
// ╚════════════════════════════════════════════════════════════╝
// Task Management REST API (VX Process-Oriented)
// =============================================================================
//
// Demonstrates a full CRUD API with in-memory storage using the VX
// Process-Oriented architecture — "break the dot-builder" pattern:
//
//   Step 1: Define domain types, store, and handlers (pure logic)
//   Step 2: Build a ServiceProcess with individual endpoint registrations
//   Step 3: Assemble into VilApp and run
//
// Endpoints:
//   GET    /api/tasks       → list all tasks
//   POST   /api/tasks       → create a new task (with validation)
//   GET    /api/tasks/:id   → get a single task by ID
//   PUT    /api/tasks/:id   → update a task by ID
//   DELETE /api/tasks/:id   → delete a task by ID
//
// In-memory storage uses Arc<RwLock<HashMap<u64, Task>>> shared via
// axum's Extension layer. The ServiceProcess compiles down to an Axum
// Router (Phase 1 bridge), so Extension injection works unchanged.
//
// Run:
//   cargo run -p basic-usage-rest-crud
//
// Test:
//   curl http://localhost:8080/api/tasks
//   curl -X POST http://localhost:8080/api/tasks \
//     -H 'Content-Type: application/json' \
//     -d '{"title":"Buy groceries","description":"Milk, eggs, bread"}'
//   curl http://localhost:8080/api/tasks/1
//   curl -X PUT http://localhost:8080/api/tasks/1 \
//     -H 'Content-Type: application/json' \
//     -d '{"title":"Buy groceries","description":"Milk, eggs, bread, butter","done":true}'
//   curl -X DELETE http://localhost:8080/api/tasks/1
// =============================================================================

use vil_server::prelude::*;
use vil_server::axum::extract::Extension;

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

// ---------------------------------------------------------------------------
// Step 1: Domain types, store, and handlers (pure logic — no framework glue)
// ---------------------------------------------------------------------------

/// A task in our TODO list.
#[derive(Debug, Clone, Serialize, Deserialize, VilModel)]
struct Task {
    id: u64,
    title: String,
    description: String,
    done: bool,
}

/// Request body for creating a task.
#[derive(Debug, Deserialize)]
struct CreateTask {
    title: String,
    #[serde(default)]
    description: String,
}

/// Request body for updating a task.
#[derive(Debug, Deserialize)]
struct UpdateTask {
    title: Option<String>,
    description: Option<String>,
    done: Option<bool>,
}

// -- Response types ----------------------------------------------------------

/// Response containing a list of tasks.
#[derive(Serialize)]
struct TaskListResponse {
    count: usize,
    tasks: Vec<Task>,
}

/// Response containing a single task with an optional message.
#[derive(Serialize)]
struct TaskResponse {
    message: &'static str,
    task: Task,
}

// -- Shared store ------------------------------------------------------------

/// In-memory task store: maps task ID to Task.
#[derive(Clone)]
struct Store {
    tasks: Arc<RwLock<HashMap<u64, Task>>>,
    next_id: Arc<std::sync::atomic::AtomicU64>,
}

impl Store {
    fn new() -> Self {
        Self {
            tasks: Arc::new(RwLock::new(HashMap::new())),
            next_id: Arc::new(std::sync::atomic::AtomicU64::new(1)),
        }
    }
}

// -- Handlers ----------------------------------------------------------------

/// GET /tasks — list all tasks.
async fn list_tasks(
    ctx: ServiceCtx,
) -> VilResponse<TaskListResponse> {
    let store = ctx.state::<Store>().expect("state type mismatch");
    let map = store.tasks.read().await;
    let tasks: Vec<Task> = map.values().cloned().collect();
    VilResponse::ok(TaskListResponse {
        count: tasks.len(),
        tasks,
    })
}

/// POST /tasks — create a new task with validation.
async fn create_task(
    ctx: ServiceCtx,
    body: ShmSlice,
) -> HandlerResult<VilResponse<TaskResponse>> {
    let store = ctx.state::<Store>().expect("state type mismatch");
    let input: CreateTask = body.json().expect("invalid JSON body");
    // Validate: title must not be empty
    if input.title.trim().is_empty() {
        return Err(VilError::bad_request("title must not be empty"));
    }

    let id = store.next_id.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    let task = Task {
        id,
        title: input.title,
        description: input.description,
        done: false,
    };

    store.tasks.write().await.insert(id, task.clone());

    Ok(VilResponse::created(TaskResponse {
        message: "Task created",
        task,
    }))
}

/// GET /tasks/:id — get a single task.
async fn get_task(
    ctx: ServiceCtx,
    Path(id): Path<u64>,
) -> HandlerResult<VilResponse<TaskResponse>> {
    let store = ctx.state::<Store>().expect("state type mismatch");
    let map = store.tasks.read().await;
    let task = map
        .get(&id)
        .cloned()
        .ok_or_else(|| VilError::not_found(format!("Task {} does not exist", id)))?;
    Ok(VilResponse::ok(TaskResponse {
        message: "Task found",
        task,
    }))
}

/// PUT /tasks/:id — update a task.
async fn update_task(
    ctx: ServiceCtx,
    Path(id): Path<u64>,
    body: ShmSlice,
) -> HandlerResult<VilResponse<TaskResponse>> {
    let store = ctx.state::<Store>().expect("state type mismatch");
    let input: UpdateTask = body.json().expect("invalid JSON body");
    let mut map = store.tasks.write().await;
    let task = map
        .get_mut(&id)
        .ok_or_else(|| VilError::not_found(format!("Task {} does not exist", id)))?;

    if let Some(title) = input.title {
        if title.trim().is_empty() {
            return Err(VilError::bad_request("title must not be empty"));
        }
        task.title = title;
    }
    if let Some(description) = input.description {
        task.description = description;
    }
    if let Some(done) = input.done {
        task.done = done;
    }

    let updated = task.clone();
    Ok(VilResponse::ok(TaskResponse {
        message: "Task updated",
        task: updated,
    }))
}

/// DELETE /tasks/:id — delete a task.
async fn delete_task(
    ctx: ServiceCtx,
    Path(id): Path<u64>,
) -> HandlerResult<VilResponse<TaskResponse>> {
    let store = ctx.state::<Store>().expect("state type mismatch");
    let mut map = store.tasks.write().await;
    let task = map
        .remove(&id)
        .ok_or_else(|| VilError::not_found(format!("Task {} does not exist", id)))?;
    Ok(VilResponse::ok(TaskResponse {
        message: "Task deleted",
        task,
    }))
}

// ---------------------------------------------------------------------------
// Step 2 + 3: Service definition and app assembly
// ---------------------------------------------------------------------------

#[tokio::main]
async fn main() {
    let store = Store::new();

    // ── Step 2: Define the Task CRUD service as a Process ──────────────
    //
    // Each endpoint is registered individually with its HTTP method, path,
    // and handler. This is the "break the dot-builder" pattern: the service
    // definition is separated from the app assembly, making each piece
    // independently testable and readable.
    //
    // The Extension(store) layer is applied to the service's built router
    // so handlers can extract Store via Extension<Store> (Phase 1 compat).

    let task_service = ServiceProcess::new("tasks")
        .prefix("/api")
        // Collection endpoints: /tasks
        .endpoint(Method::GET,    "/tasks", get(list_tasks))
        .endpoint(Method::POST,   "/tasks", post(create_task))
        // Item endpoints: /tasks/:id
        .endpoint(Method::GET,    "/tasks/:id", get(get_task))
        .endpoint(Method::PUT,    "/tasks/:id", put(update_task))
        .endpoint(Method::DELETE, "/tasks/:id", delete(delete_task))
        // Inject the in-memory store so handlers can extract via Extension<Store>
        .state(store);

    // ── Step 3: Assemble into VilApp and run ────────────────────────
    //
    // VilApp composes services into a process topology and delegates
    // the HTTP boundary to VilServer (Phase 1 bridge). Each service
    // gets its own prefix, and the VX banner prints the full topology.

    VilApp::new("crud-service")
        .port(8080)
        .service(task_service)
        .run()
        .await;
}
