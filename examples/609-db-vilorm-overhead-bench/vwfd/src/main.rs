// 609 — VilORM Overhead Benchmark (VWFD)
// Business logic identical to standard:
//   GET /orm/items, /orm/items/:id, /orm/count, /orm/cols
//   (raw/* endpoints mirror orm/* — VWFD only has ORM path)
use serde_json::{json, Value};

fn orm_find_by_id(input: &Value) -> Result<Value, String> {
    let path = input["path"].as_str().unwrap_or("");
    let id = path.split('/').last().unwrap_or("1");
    Ok(json!({"id": id, "name": "Benchmark Item", "value": 42.5, "category": "test", "created_at": "2024-01-15T10:00:00Z"}))
}

fn orm_list(_input: &Value) -> Result<Value, String> {
    let items: Vec<Value> = (1..=10).map(|i| json!({"id": i, "name": format!("Item {}", i), "value": i as f64 * 10.5, "category": "test", "created_at": "2024-01-15T10:00:00Z"})).collect();
    Ok(json!(items))
}

fn orm_count(_input: &Value) -> Result<Value, String> {
    Ok(json!({"count": 1000}))
}

fn orm_cols(_input: &Value) -> Result<Value, String> {
    let items: Vec<Value> = (1..=10).map(|i| json!({"id": i, "name": format!("Item {}", i)})).collect();
    Ok(json!(items))
}

#[tokio::main]
async fn main() {
    vil_vwfd::app("examples/609-db-vilorm-overhead-bench/vwfd/workflows", 3249)
        .native("orm_find_by_id", orm_find_by_id)
        .native("orm_list", orm_list)
        .native("orm_count", orm_count)
        .native("orm_cols", orm_cols)
        .run().await;
}
