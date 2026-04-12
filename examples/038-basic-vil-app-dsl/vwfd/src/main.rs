// 038 — Restaurant Order System (Hybrid: WASM AssemblyScript for order processing, NativeCode for status)
use serde_json::json;

#[tokio::main]
async fn main() {
    vil_vwfd::app("examples/038-basic-vil-app-dsl/vwfd/workflows", 8080)
        // Menu listing — NativeCode (static data)
        .native("menu_handler", |_| {
            Ok(json!({
                "restaurant": "VIL Bistro",
                "items": [
                    {"name": "Nasi Goreng", "price_cents": 35000},
                    {"name": "Soto Ayam", "price_cents": 28000},
                    {"name": "Rendang", "price_cents": 45000},
                ]
            }))
        })
        // Order creation with price calc — WASM AssemblyScript (sandboxed computation)
        .wasm("order_create", "examples/038-basic-vil-app-dsl/vwfd/wasm/assemblyscript/restaurant.wasm")
        // Order status lookup — NativeCode (simple ID extraction)
        .native("order_status", |input| {
            let path = input["path"].as_str().unwrap_or("");
            let id = path.split('/').last().unwrap_or("42");
            Ok(json!({"order_id": format!("ORD-{}", id), "status": "preparing", "eta_minutes": 10}))
        })
        // Kitchen status — NativeCode (static mock)
        .native("kitchen_status", |_| {
            Ok(json!({"chefs_on_duty": 3, "kitchen_load_percent": 65, "orders_in_queue": 4}))
        })
        .run().await;
}
