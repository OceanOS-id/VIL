// 038 — Restaurant Order System (VWFD)
// Business logic identical to standard:
//   - Restaurant: "Trattoria VIL — Italian Kitchen"
//   - 7 menu items: Bruschetta(899), Caesar Salad(1299), Margherita Pizza(1599),
//     Spaghetti Carbonara(1899), Grilled Salmon(2499), Tiramisu(899), Panna Cotta(799)
//   - Order: validate items, calculate total, generate order_id
//   - Kitchen: orders_in_queue, active_chefs, avg_wait_minutes
use serde_json::{json, Value};
use std::sync::atomic::{AtomicU64, Ordering};

static ORDER_COUNTER: AtomicU64 = AtomicU64::new(1);

const MENU: &[(&str, u64, &str)] = &[
    ("Bruschetta", 899, "appetizer"),
    ("Caesar Salad", 1299, "appetizer"),
    ("Margherita Pizza", 1599, "main"),
    ("Spaghetti Carbonara", 1899, "main"),
    ("Grilled Salmon", 2499, "main"),
    ("Tiramisu", 899, "dessert"),
    ("Panna Cotta", 799, "dessert"),
];

fn menu_handler(_input: &Value) -> Result<Value, String> {
    let items: Vec<Value> = MENU.iter().map(|(name, price, cat)| {
        json!({"name": name, "price_cents": price, "category": cat})
    }).collect();
    Ok(json!({
        "restaurant": "Trattoria VIL — Italian Kitchen",
        "items": items
    }))
}

fn order_handler(input: &Value) -> Result<Value, String> {
    let body = input.get("body").cloned().unwrap_or(json!({}));
    let table = body["table_number"].as_u64().unwrap_or(0);
    let items = body["items"].as_array();

    if items.is_none() || items.unwrap().is_empty() {
        return Ok(json!({"error": "Order must contain at least one item", "status": 400}));
    }

    let mut total = 0u64;
    let mut order_items = Vec::new();
    for item_name in items.unwrap() {
        let name = item_name.as_str().unwrap_or("");
        if let Some((n, price, cat)) = MENU.iter().find(|(n, _, _)| *n == name) {
            total += price;
            order_items.push(json!({"name": n, "price_cents": price, "category": cat}));
        }
    }

    let order_id = format!("ORD-{:04}", ORDER_COUNTER.fetch_add(1, Ordering::Relaxed));
    Ok(json!({
        "order_id": order_id,
        "table_number": table,
        "items": order_items,
        "total_cents": total,
        "status": "confirmed — kitchen notified"
    }))
}

fn order_status(input: &Value) -> Result<Value, String> {
    let path = input.get("path").and_then(|v| v.as_str()).unwrap_or("");
    let id = path.split('/').last().unwrap_or("0001");
    Ok(json!({
        "order_id": id,
        "status": "cooking — your pizza is in the oven",
        "progress_percent": 65,
        "estimated_remaining_minutes": 8
    }))
}

fn kitchen_status(_input: &Value) -> Result<Value, String> {
    Ok(json!({
        "orders_in_queue": 5,
        "active_chefs": 3,
        "avg_wait_minutes": 12,
        "peak_hour": false
    }))
}

#[tokio::main]
async fn main() {
    vil_vwfd::app("examples/038-basic-vil-app-dsl/vwfd/workflows", 8080)
        .native("menu_handler", menu_handler)
        .native("order_create", order_handler)
        .native("order_status", order_status)
        .native("kitchen_status", kitchen_status)
        .run().await;
}
