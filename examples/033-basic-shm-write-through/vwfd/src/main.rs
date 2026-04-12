// 033 — SHM Write-Through (Product Catalog)
use serde_json::json;

#[tokio::main]
async fn main() {
    vil_vwfd::app("examples/033-basic-shm-write-through/vwfd/workflows", 8080)
        .native("catalog_health_handler", |_| {
            Ok(json!({"status": "healthy", "service": "catalog", "shm_available": true}))
        })
        .native("catalog_search_handler", |input| {
            let body = input.get("body").cloned().unwrap_or(json!({}));
            let category = body.get("category").and_then(|v| v.as_str()).unwrap_or("all");
            let max_price = body.get("max_price_cents").and_then(|v| v.as_u64()).unwrap_or(999999);
            let products = vec![
                json!({"id": "P-001", "name": "Wireless Mouse", "category": "electronics", "price_cents": 2999}),
                json!({"id": "P-002", "name": "USB-C Hub", "category": "electronics", "price_cents": 4599}),
            ];
            let filtered: Vec<_> = products.into_iter().filter(|p| {
                let cat_match = category == "all" || p["category"].as_str() == Some(category);
                let price_match = p["price_cents"].as_u64().unwrap_or(0) <= max_price;
                cat_match && price_match
            }).collect();
            let count = filtered.len();
            Ok(json!({
                "products": filtered,
                "products_returned": count,
                "shm_available": true,
                "cache_hit": false
            }))
        })
        .run()
        .await;
}
