// 607 — Multi-Tenant SaaS (VWFD)
// Business logic identical to standard:
//   POST /tenants, GET/PUT /tenants/:id, POST/GET /tenants/:id/users,
//   POST/GET /tenants/:id/settings, GET /tenants/:id/stats
// Response: Tenant { id, name, plan, is_active, created_at }
use serde_json::{json, Value};
use std::sync::atomic::{AtomicU64, Ordering};

static TENANT_SEQ: AtomicU64 = AtomicU64::new(1);
static USER_SEQ: AtomicU64 = AtomicU64::new(1);

fn create_tenant(input: &Value) -> Result<Value, String> {
    let body = &input["body"];
    let id = format!("T-{:04}", TENANT_SEQ.fetch_add(1, Ordering::Relaxed));
    Ok(json!({
        "_status": 201,
        "id": id,
        "name": body["name"].as_str().unwrap_or("New Tenant"),
        "plan": body["plan"].as_str().unwrap_or("starter"),
        "is_active": true,
        "created_at": "2024-01-15T10:00:00Z"
    }))
}

fn get_tenant(input: &Value) -> Result<Value, String> {
    let path = input["path"].as_str().unwrap_or("");
    let id = path.split('/').last().unwrap_or("T-0001");
    Ok(json!({
        "id": id,
        "name": "Acme Corp",
        "plan": "professional",
        "is_active": true,
        "created_at": "2024-01-10T08:00:00Z"
    }))
}

fn update_tenant(input: &Value) -> Result<Value, String> {
    let body = &input["body"];
    let path = input["path"].as_str().unwrap_or("");
    let id = path.split('/').last().unwrap_or("T-0001");
    Ok(json!({
        "id": id,
        "name": body["name"].as_str().unwrap_or("Acme Corp"),
        "plan": body["plan"].as_str().unwrap_or("professional"),
        "is_active": body["is_active"].as_bool().unwrap_or(true),
        "created_at": "2024-01-10T08:00:00Z"
    }))
}

fn add_user(input: &Value) -> Result<Value, String> {
    let body = &input["body"];
    let path = input["path"].as_str().unwrap_or("");
    let parts: Vec<&str> = path.split('/').collect();
    let tenant_id = parts.iter().rev().nth(1).unwrap_or(&"T-0001");
    Ok(json!({
        "ok": true,
        "tenant_id": tenant_id,
        "email": body["email"].as_str().unwrap_or("user@example.com")
    }))
}

fn list_users(input: &Value) -> Result<Value, String> {
    let path = input["path"].as_str().unwrap_or("");
    let parts: Vec<&str> = path.split('/').collect();
    let tenant_id = parts.iter().rev().nth(1).unwrap_or(&"T-0001");
    Ok(json!([
        {"id": "U-0001", "tenant_id": tenant_id, "name": "Alice", "email": "alice@acme.com", "role": "admin"},
        {"id": "U-0002", "tenant_id": tenant_id, "name": "Bob", "email": "bob@acme.com", "role": "user"}
    ]))
}

fn upsert_setting(input: &Value) -> Result<Value, String> {
    let body = &input["body"];
    let path = input["path"].as_str().unwrap_or("");
    let parts: Vec<&str> = path.split('/').collect();
    let tenant_id = parts.iter().rev().nth(1).unwrap_or(&"T-0001");
    Ok(json!({
        "ok": true,
        "tenant_id": tenant_id,
        "key": body["key"].as_str().unwrap_or("theme"),
        "value": body["value"].as_str().unwrap_or("dark")
    }))
}

fn list_settings(input: &Value) -> Result<Value, String> {
    let path = input["path"].as_str().unwrap_or("");
    let parts: Vec<&str> = path.split('/').collect();
    let tenant_id = parts.iter().rev().nth(1).unwrap_or(&"T-0001");
    Ok(json!([
        {"tenant_id": tenant_id, "key": "theme", "value": "dark"},
        {"tenant_id": tenant_id, "key": "language", "value": "id"},
        {"tenant_id": tenant_id, "key": "timezone", "value": "Asia/Jakarta"}
    ]))
}

fn tenant_stats(input: &Value) -> Result<Value, String> {
    let path = input["path"].as_str().unwrap_or("");
    let parts: Vec<&str> = path.split('/').collect();
    let tenant_id = parts.iter().rev().nth(1).unwrap_or(&"T-0001");
    Ok(json!({
        "tenant_id": tenant_id,
        "tenant_name": "Acme Corp",
        "plan": "professional",
        "is_active": true,
        "user_count": 2,
        "setting_count": 3
    }))
}

#[tokio::main]
async fn main() {
    vil_vwfd::app("examples/607-db-vilorm-multitenant/vwfd/workflows", 8087)
        .native("create_tenant", create_tenant)
        .native("get_tenant", get_tenant)
        .native("update_tenant", update_tenant)
        .native("add_user", add_user)
        .native("list_users", list_users)
        .native("upsert_setting", upsert_setting)
        .native("list_settings", list_settings)
        .native("tenant_stats", tenant_stats)
        .run().await;
}
