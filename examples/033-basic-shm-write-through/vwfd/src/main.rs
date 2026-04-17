// 033 — SHM Write-Through Product Catalog (VWFD)
// Business logic identical to standard:
//   - 6 products across electronics/furniture categories
//   - Search: filter by category + max_price_cents
//   - Response: products, products_returned, category_searched, shm_available
use serde_json::{json, Value};

fn catalog() -> Vec<Value> {
    vec![
        json!({"product_id": 1001, "name": "Wireless Mouse", "category": "electronics", "price_cents": 2999, "stock_count": 150}),
        json!({"product_id": 1002, "name": "USB-C Hub", "category": "electronics", "price_cents": 4999, "stock_count": 80}),
        json!({"product_id": 1003, "name": "4K Monitor", "category": "electronics", "price_cents": 34999, "stock_count": 25}),
        json!({"product_id": 2001, "name": "Standing Desk", "category": "furniture", "price_cents": 59999, "stock_count": 12}),
        json!({"product_id": 2002, "name": "Ergonomic Chair", "category": "furniture", "price_cents": 44999, "stock_count": 30}),
    ]
}

fn catalog_search(input: &Value) -> Result<Value, String> {
    let body = input.get("body").cloned().unwrap_or(json!({}));
    let category = body["category"].as_str().unwrap_or("electronics");
    let max_price = body["max_price_cents"].as_i64().unwrap_or(999999);

    let products: Vec<Value> = catalog().into_iter()
        .filter(|p| p["category"].as_str() == Some(category) && p["price_cents"].as_i64().unwrap_or(0) <= max_price)
        .collect();

    Ok(json!({
        "products": products,
        "products_returned": products.len(),
        "category_searched": category,
        "shm_available": true,
        "analytics_note": "Write-through: response also written to ExchangeHeap for co-located analytics"
    }))
}

fn catalog_health(_input: &Value) -> Result<Value, String> {
    Ok(json!({
        "products": [],
        "products_returned": 0,
        "category_searched": "n/a",
        "shm_available": true,
        "analytics_note": "Health check — plain response without SHM write-through"
    }))
}

#[tokio::main]
async fn main() {
    vil_vwfd::app("examples/033-basic-shm-write-through/vwfd/workflows", 8080)
        .native("catalog_health_handler", catalog_health)
        .native("catalog_search_handler", catalog_search)
        .run().await;
}
