// ╔════════════════════════════════════════════════════════════╗
// ║  004 — Task CRUD with SQLite + VilORM                     ║
// ╠════════════════════════════════════════════════════════════╣
// ║  Pattern:  VX_APP + VilEntity + VilQuery                    ║
// ║  Features: VilORM, VilQuery builder, VilEntity derive       ║
// ║  Domain:   Task management — full CRUD on SQLite            ║
// ╠════════════════════════════════════════════════════════════╣
// ║  Showcases:                                                 ║
// ║  - #[derive(VilEntity)] for auto CRUD methods               ║
// ║  - T::q() fluent query builder (JOOQ-style)                ║
// ║  - T::find_by_id() / T::find_all() convenience methods     ║
// ║  - T::q().insert_columns().value().execute() for INSERT     ║
// ║  - T::q().update().set_optional().where_eq() for UPDATE     ║
// ║  - T::q().select(&[cols]).order_by_desc() for projections   ║
// ║  - T::delete() for DELETE by primary key                    ║
// ╚════════════════════════════════════════════════════════════╝
//
// Run:   cargo run -p vil-basic-rest-crud
// Test:
//   curl http://localhost:8080/api/tasks
//   curl -X POST http://localhost:8080/api/tasks \
//     -H 'Content-Type: application/json' \
//     -d '{"title":"Buy groceries","description":"Milk, eggs, bread"}'
//   curl http://localhost:8080/api/tasks/some-uuid-here
//   curl -X PUT http://localhost:8080/api/tasks/some-uuid-here \
//     -H 'Content-Type: application/json' \
//     -d '{"title":"Buy groceries","description":"Updated list","done":true}'
//   curl -X DELETE http://localhost:8080/api/tasks/some-uuid-here

use serde::{Deserialize, Serialize};
use std::sync::Arc;
use vil_db_sqlx::SqlxPool;
use vil_orm::VilQuery;
use vil_orm_derive::VilEntity;
use vil_server::prelude::*;

// ── Domain Model ──

#[derive(Debug, Clone, Serialize, Deserialize, VilModel, sqlx::FromRow, VilEntity)]
#[vil_entity(table = "tasks")]
struct Task {
    #[vil_entity(pk)]
    id: String,
    title: String,
    description: String,
    done: i64,
    #[vil_entity(auto_now_add)]
    created_at: String,
    #[vil_entity(auto_now)]
    updated_at: String,
}

#[derive(Debug, Deserialize)]
struct CreateTask {
    title: String,
    #[serde(default)]
    description: Option<String>,
}

#[derive(Debug, Deserialize)]
struct UpdateTask {
    title: Option<String>,
    description: Option<String>,
    done: Option<bool>,
}

// Slim projection — only fetch what the list endpoint needs
#[derive(Debug, Clone, Serialize, Deserialize, VilModel, sqlx::FromRow)]
struct TaskListItem {
    id: String,
    title: String,
    done: i64,
    created_at: String,
}

// ── Shared State ──

#[derive(Clone)]
struct AppState {
    pool: Arc<SqlxPool>,
}

// ── Handlers ──

/// GET /tasks — list tasks (slim projection, not SELECT *)
async fn list_tasks(ctx: ServiceCtx) -> HandlerResult<VilResponse<Vec<TaskListItem>>> {
    let state = ctx.state::<AppState>().expect("state");
    let tasks = Task::q()
        .select(&["id", "title", "done", "created_at"])
        .order_by_desc("created_at")
        .limit(100)
        .fetch_all::<TaskListItem>(state.pool.inner())
        .await
        .map_err(|e| VilError::internal(format!("{e}")))?;
    Ok(VilResponse::ok(tasks))
}

/// POST /tasks — create task via VilQuery insert
async fn create_task(ctx: ServiceCtx, body: ShmSlice) -> HandlerResult<VilResponse<Task>> {
    let state = ctx.state::<AppState>().expect("state");
    let req: CreateTask = body.json().map_err(|_| VilError::bad_request("invalid JSON"))?;

    if req.title.trim().is_empty() {
        return Err(VilError::bad_request("title must not be empty"));
    }

    let id = uuid::Uuid::new_v4().to_string();
    let title = req.title;
    let desc = req.description.unwrap_or_default();

    Task::q()
        .insert_columns(&["id", "title", "description"])
        .value(id.clone())
        .value(title)
        .value(desc)
        .execute(state.pool.inner())
        .await
        .map_err(|e| VilError::internal(format!("{e}")))?;

    // Fetch back full record
    let task = Task::find_by_id(state.pool.inner(), &id)
        .await
        .map_err(|e| VilError::internal(format!("{e}")))?
        .ok_or_else(|| VilError::internal("insert succeeded but fetch failed"))?;

    Ok(VilResponse::created(task))
}

/// GET /tasks/:id — get by primary key
async fn get_task(ctx: ServiceCtx, Path(id): Path<String>) -> HandlerResult<VilResponse<Task>> {
    let state = ctx.state::<AppState>().expect("state");
    let task = Task::find_by_id(state.pool.inner(), &id)
        .await
        .map_err(|e| VilError::internal(format!("{e}")))?
        .ok_or_else(|| VilError::not_found(format!("Task {id} not found")))?;
    Ok(VilResponse::ok(task))
}

/// PUT /tasks/:id — partial update via VilQuery set_optional (skip None fields)
async fn update_task(
    ctx: ServiceCtx,
    Path(id): Path<String>,
    body: ShmSlice,
) -> HandlerResult<VilResponse<Task>> {
    let state = ctx.state::<AppState>().expect("state");
    let req: UpdateTask = body.json().map_err(|_| VilError::bad_request("invalid JSON"))?;

    // Validate title if provided
    if let Some(ref t) = req.title {
        if t.trim().is_empty() {
            return Err(VilError::bad_request("title must not be empty"));
        }
    }

    // Build UPDATE dynamically — only SET provided fields
    let mut q = Task::q()
        .update()
        .set_optional("title", req.title.as_deref())
        .set_optional("description", req.description.as_deref())
        .set_raw("updated_at", "datetime('now')");

    if let Some(done) = req.done {
        q = q.set("done", if done { 1_i64 } else { 0_i64 });
    }

    q.where_eq("id", &id)
        .execute(state.pool.inner())
        .await
        .map_err(|e| VilError::internal(format!("{e}")))?;

    let task = Task::find_by_id(state.pool.inner(), &id)
        .await
        .map_err(|e| VilError::internal(format!("{e}")))?
        .ok_or_else(|| VilError::not_found(format!("Task {id} not found")))?;

    Ok(VilResponse::ok(task))
}

/// DELETE /tasks/:id — delete by primary key
async fn delete_task(ctx: ServiceCtx, Path(id): Path<String>) -> HandlerResult<VilResponse<Task>> {
    let state = ctx.state::<AppState>().expect("state");
    let task = Task::find_by_id(state.pool.inner(), &id)
        .await
        .map_err(|e| VilError::internal(format!("{e}")))?
        .ok_or_else(|| VilError::not_found(format!("Task {id} not found")))?;

    Task::delete(state.pool.inner(), &id)
        .await
        .map_err(|e| VilError::internal(format!("{e}")))?;

    Ok(VilResponse::ok(task))
}

/// GET /tasks/stats — aggregate via VilQuery scalar
async fn task_stats(ctx: ServiceCtx) -> HandlerResult<VilResponse<serde_json::Value>> {
    let state = ctx.state::<AppState>().expect("state");
    let pool = state.pool.inner();

    let total = Task::count(pool)
        .await
        .map_err(|e| VilError::internal(format!("{e}")))?;

    let done: i64 = Task::q()
        .select_expr("CAST(COUNT(*) AS INTEGER)")
        .where_eq_val("done", 1_i64)
        .scalar::<i64>(pool)
        .await
        .unwrap_or(0);

    Ok(VilResponse::ok(serde_json::json!({
        "total": total,
        "done": done,
        "pending": total - done,
    })))
}

// ── Main ──

#[tokio::main]
async fn main() {
    let db_url = std::env::var("DATABASE_URL").unwrap_or_else(|_| "sqlite:tasks.db".into());

    // Connect via VIL SqlxPool
    let pool = vil_db_sqlx::SqlxPool::connect(
        "tasks",
        vil_db_sqlx::SqlxConfig::sqlite(&db_url),
    )
    .await
    .expect("Failed to connect to SQLite");

    // Auto-create table
    pool.execute_raw(
        "CREATE TABLE IF NOT EXISTS tasks (
            id TEXT PRIMARY KEY,
            title TEXT NOT NULL,
            description TEXT DEFAULT '',
            done INTEGER DEFAULT 0,
            created_at TEXT DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),
            updated_at TEXT DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))
        )"
    ).await.expect("Migration failed");

    let state = AppState {
        pool: Arc::new(pool),
    };

    let task_svc = ServiceProcess::new("tasks")
        .endpoint(Method::GET, "/tasks", get(list_tasks))
        .endpoint(Method::POST, "/tasks", post(create_task))
        .endpoint(Method::GET, "/tasks/stats", get(task_stats))
        .endpoint(Method::GET, "/tasks/:id", get(get_task))
        .endpoint(Method::PUT, "/tasks/:id", put(update_task))
        .endpoint(Method::DELETE, "/tasks/:id", delete(delete_task))
        .state(state);

    VilApp::new("crud-vilorm")
        .port(8080)
        .observer(true)
        .service(task_svc)
        .run()
        .await;
}
