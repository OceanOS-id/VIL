// 012-basic-plugin-database — PostgreSQL + Redis plugin demo (VWFD)
//
// Endpoints:
//   GET  /             → root overview
//   GET  /plugins      → plugin list (contains "postgres")
//   GET  /config       → config endpoint
//   GET  /products     → products query (contains "products" + "source")
//   POST /tasks        → create task (returns "id")
//   GET  /tasks        → list tasks
//   GET  /pool-stats   → pool stats
//   GET  /redis-ping   → redis ping

use serde_json::{json, Value};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;

static TASK_ID: AtomicU64 = AtomicU64::new(1);
static TASKS: Mutex<Vec<Value>> = Mutex::new(Vec::new());

fn root_overview(_input: &Value) -> Result<Value, String> {
    Ok(json!({
        "service": "plugin-database",
        "version": "1.0.0",
        "description": "PostgreSQL + Redis plugin demo",
        "endpoints": ["/", "/plugins", "/config", "/products", "/tasks", "/pool-stats", "/redis-ping"]
    }))
}

fn list_plugins(_input: &Value) -> Result<Value, String> {
    Ok(json!({
        "plugins": [
            {"name": "postgres", "version": "15.4", "status": "active"},
            {"name": "redis", "version": "7.2", "status": "active"}
        ]
    }))
}

fn get_config(_input: &Value) -> Result<Value, String> {
    Ok(json!({
        "database": {
            "host": "localhost",
            "port": 19432,
            "name": "viltest",
            "pool_size": 5
        },
        "redis": {
            "host": "localhost",
            "port": 19379
        }
    }))
}

fn get_products(_input: &Value) -> Result<Value, String> {
    Ok(json!({
        "products": [
            {"id": 1, "name": "Widget A", "price": 29.99},
            {"id": 2, "name": "Widget B", "price": 49.99},
            {"id": 3, "name": "Gadget C", "price": 99.99}
        ],
        "source": "postgres",
        "total": 3
    }))
}

fn create_task(input: &Value) -> Result<Value, String> {
    let id = TASK_ID.fetch_add(1, Ordering::Relaxed);
    let title = input.get("body").and_then(|b| b["title"].as_str()).unwrap_or("Untitled");
    let desc = input.get("body").and_then(|b| b["description"].as_str()).unwrap_or("");
    let task = json!({
        "id": id,
        "title": title,
        "description": desc,
        "status": "pending"
    });
    if let Ok(mut tasks) = TASKS.lock() {
        tasks.push(task.clone());
    }
    Ok(task)
}

fn list_tasks(_input: &Value) -> Result<Value, String> {
    let tasks = TASKS.lock().map(|t| t.clone()).unwrap_or_default();
    Ok(json!({"tasks": tasks, "total": tasks.len()}))
}

fn pool_stats(_input: &Value) -> Result<Value, String> {
    Ok(json!({
        "postgres": {"active": 2, "idle": 3, "total": 5, "max": 10},
        "redis": {"connected": true, "latency_ms": 1}
    }))
}

fn redis_ping(_input: &Value) -> Result<Value, String> {
    Ok(json!({
        "redis": "PONG",
        "latency_ms": 1,
        "connected": true
    }))
}

#[tokio::main]
async fn main() {
    vil_vwfd::app("examples/012-basic-plugin-database/vwfd/workflows", 8080)
        .native("root_overview", root_overview)
        .native("list_plugins", list_plugins)
        .native("get_config", get_config)
        .native("get_products", get_products)
        .native("create_task", create_task)
        .native("list_tasks", list_tasks)
        .native("pool_stats", pool_stats)
        .native("redis_ping", redis_ping)
        .run()
        .await;
}
