# Recipe: REST CRUD

Complete CRUD service with ServiceCtx, ShmSlice, and typed state. Copy-paste ready.

## Full Example

```rust
use vil_server::prelude::*;
use std::sync::Arc;
use tokio::sync::RwLock;
use std::collections::HashMap;

#[derive(Clone, Debug, Serialize, Deserialize, VilModel)]
struct Task {
    id: u64,
    title: String,
    done: bool,
}

#[derive(Clone, Debug, Deserialize)]
struct CreateTask {
    title: String,
}

type TaskStore = Arc<RwLock<HashMap<u64, Task>>>;

#[tokio::main]
async fn main() {
    let store: TaskStore = Arc::new(RwLock::new(HashMap::new()));

    let service = ServiceProcess::new("tasks")
        .prefix("/api")
        .extension(store)
        .endpoint(Method::GET, "/tasks", get(list_tasks))
        .endpoint(Method::POST, "/tasks", post(create_task))
        .endpoint(Method::GET, "/tasks/:id", get(get_task))
        .endpoint(Method::PUT, "/tasks/:id", put(update_task))
        .endpoint(Method::DELETE, "/tasks/:id", delete(delete_task));

    VilApp::new("crud-service")
        .port(8080)
        .service(service)
        .run()
        .await;
}
```

## List (GET /api/tasks)

```rust
async fn list_tasks(ctx: ServiceCtx) -> VilResponse<Vec<Task>> {
    let store = ctx.state::<TaskStore>();
    let tasks: Vec<Task> = store.read().await.values().cloned().collect();
    VilResponse::ok(tasks)
}
```

## Create (POST /api/tasks)

```rust
#[vil_handler(shm)]
async fn create_task(ctx: ServiceCtx, slice: ShmSlice) -> VilResponse<Task> {
    let input: CreateTask = slice.json()?;
    let mut store = ctx.state::<TaskStore>().write().await;
    let id = store.len() as u64 + 1;
    let task = Task { id, title: input.title, done: false };
    store.insert(id, task.clone());
    VilResponse::created(task)
}
```

## Read (GET /api/tasks/:id)

```rust
async fn get_task(ctx: ServiceCtx, Path(id): Path<u64>) -> HandlerResult<VilResponse<Task>> {
    let store = ctx.state::<TaskStore>();
    let task = store.read().await.get(&id).cloned()
        .ok_or_else(|| VilError::not_found(format!("Task {} not found", id)))?;
    Ok(VilResponse::ok(task))
}
```

## Update (PUT /api/tasks/:id)

```rust
#[vil_handler(shm)]
async fn update_task(
    ctx: ServiceCtx, Path(id): Path<u64>, slice: ShmSlice,
) -> HandlerResult<VilResponse<Task>> {
    let input: CreateTask = slice.json()?;
    let mut store = ctx.state::<TaskStore>().write().await;
    let task = store.get_mut(&id)
        .ok_or_else(|| VilError::not_found(format!("Task {} not found", id)))?;
    task.title = input.title;
    Ok(VilResponse::ok(task.clone()))
}
```

## Delete (DELETE /api/tasks/:id)

```rust
async fn delete_task(ctx: ServiceCtx, Path(id): Path<u64>) -> HandlerResult<VilResponse<()>> {
    let mut store = ctx.state::<TaskStore>().write().await;
    store.remove(&id)
        .ok_or_else(|| VilError::not_found(format!("Task {} not found", id)))?;
    Ok(VilResponse::ok(()))
}
```

## Test with curl

```bash
curl -X POST http://localhost:8080/api/tasks -d '{"title":"Learn VIL"}'
curl http://localhost:8080/api/tasks
curl http://localhost:8080/api/tasks/1
curl -X PUT http://localhost:8080/api/tasks/1 -d '{"title":"Master VIL"}'
curl -X DELETE http://localhost:8080/api/tasks/1
```

> Reference: docs/vil/003-VIL-Developer_Guide-Server-Framework.md
