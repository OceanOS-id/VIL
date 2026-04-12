// 606 — E-commerce (Mixed: VilQuery YAML + NativeCode with VilQuery builder)
use serde_json::json;
use std::sync::{Arc, OnceLock};
use vil_orm::VilQuery;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, sqlx::FromRow)]
struct Product { id: String, name: String, price: f64, stock: i64, category: String, created_at: String }

fn uuid() -> String {
    use std::sync::atomic::{AtomicU64, Ordering};
    static C: AtomicU64 = AtomicU64::new(1);
    let t = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos();
    let c = C.fetch_add(1, Ordering::Relaxed);
    format!("{:08x}-{:04x}-4{:03x}-{:04x}-{:012x}",
        (t >> 32) as u32, (t >> 16) as u16 & 0xffff, t as u16 & 0x0fff,
        0x8000 | (c as u16 & 0x3fff), t as u64 & 0xffffffffffff)
}

static POOL: OnceLock<Arc<vil_db_sqlx::SqlxPool>> = OnceLock::new();
fn pool() -> &'static Arc<vil_db_sqlx::SqlxPool> { POOL.get().expect("pool") }

fn block_async<F: std::future::Future>(f: F) -> F::Output {
    tokio::task::block_in_place(|| tokio::runtime::Handle::current().block_on(f))
}

#[tokio::main]
async fn main() {
    if std::env::var("VIL_DATABASE_URL").is_err() {
        let p = format!("{}/shop_vwfd.db", std::env::temp_dir().display());
        std::env::set_var("VIL_DATABASE_URL", format!("sqlite:{}?mode=rwc", p));
    }
    let url = std::env::var("VIL_DATABASE_URL").unwrap();
    let db = vil_db_sqlx::SqlxPool::connect("shop", vil_db_sqlx::SqlxConfig::sqlite(&url))
        .await.expect("db");
    db.execute_raw("CREATE TABLE IF NOT EXISTS products (id TEXT PRIMARY KEY, name TEXT NOT NULL, description TEXT, price REAL NOT NULL, stock INTEGER DEFAULT 0, category TEXT DEFAULT '', created_at TEXT DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ','now')))").await.ok();
    db.execute_raw("CREATE TABLE IF NOT EXISTS orders (id TEXT PRIMARY KEY, customer_name TEXT NOT NULL, status TEXT DEFAULT 'pending', created_at TEXT DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ','now')))").await.ok();
    db.execute_raw("CREATE TABLE IF NOT EXISTS order_items (id INTEGER PRIMARY KEY AUTOINCREMENT, order_id TEXT, product_id TEXT, quantity INTEGER DEFAULT 1)").await.ok();
    POOL.set(Arc::new(db)).unwrap_or_else(|_| panic!("pool"));

    vil_vwfd::app("examples/606-db-vilorm-ecommerce/vwfd/workflows", 8086)
        .native("create_product", |input| {
            let body = &input["body"];
            let id = uuid();
            let name = body["name"].as_str().unwrap_or("Item").to_string();
            let desc = body["description"].as_str().unwrap_or("").to_string();
            let price = body["price"].as_f64().unwrap_or(0.0);
            let stock = body["stock"].as_i64().unwrap_or(0);
            let cat = body["category"].as_str().unwrap_or("").to_string();
            block_async(async {
                VilQuery::new("products")
                    .insert_columns(&["id", "name", "description", "price", "stock", "category"])
                    .value(id.clone()).value(name.clone()).value(desc)
                    .value(price).value(stock).value(cat)
                    .execute(pool().inner()).await
                    .map_err(|e| format!("500:{}", e))?;
                let p = VilQuery::new("products").where_eq("id", &id)
                    .fetch_one::<Product>(pool().inner()).await
                    .map_err(|e| format!("500:{}", e))?;
                Ok(json!({"_status": 201, "id": p.id, "name": p.name, "price": p.price, "stock": p.stock}))
            })
        })
        .native("create_order", |input| {
            let body = &input["body"];
            let id = uuid();
            let customer = body["customer_name"].as_str().unwrap_or("Guest").to_string();
            let items = body["items"].as_array().cloned().unwrap_or_default();
            block_async(async {
                VilQuery::new("orders")
                    .insert_columns(&["id", "customer_name"])
                    .value(id.clone()).value(customer.clone())
                    .execute(pool().inner()).await
                    .map_err(|e| format!("500:{}", e))?;
                for item in &items {
                    let prod_id = item["product_id"].as_str().unwrap_or("").to_string();
                    let qty = item["quantity"].as_i64().unwrap_or(1);
                    VilQuery::new("order_items")
                        .insert_columns(&["order_id", "product_id", "quantity"])
                        .value(id.clone()).value(prod_id).value(qty)
                        .execute(pool().inner()).await.ok();
                }
                Ok(json!({"_status": 201, "id": id, "customer_name": customer, "items_count": items.len(), "status": "pending"}))
            })
        })
        .run().await;
}
