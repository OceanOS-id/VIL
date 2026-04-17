// 606 — E-commerce (Pure VilQuery Workflow — no NativeCode DB access)
//
// ALL DB operations via VilQuery Connector in workflow YAML.
// Compatible with provision mode (no .so needed for DB ops).

#[tokio::main]
async fn main() {
    if std::env::var("VIL_DATABASE_URL").is_err() {
        let p = format!("{}/shop_vwfd.db", std::env::temp_dir().display());
        std::env::set_var("VIL_DATABASE_URL", format!("sqlite:{}?mode=rwc", p));
    }
    let url = std::env::var("VIL_DATABASE_URL").unwrap();
    let db = vil_db_sqlx::SqlxPool::connect("shop", vil_db_sqlx::SqlxConfig::sqlite(&url))
        .await.expect("db connect");

    db.execute_raw("CREATE TABLE IF NOT EXISTS products (id TEXT PRIMARY KEY, name TEXT NOT NULL, description TEXT, price REAL NOT NULL, stock INTEGER DEFAULT 0, category TEXT DEFAULT '', created_at TEXT DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ','now')))").await.ok();
    db.execute_raw("CREATE TABLE IF NOT EXISTS orders (id TEXT PRIMARY KEY, customer_name TEXT NOT NULL, status TEXT DEFAULT 'pending', created_at TEXT DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ','now')))").await.ok();
    db.execute_raw("CREATE TABLE IF NOT EXISTS order_items (id INTEGER PRIMARY KEY AUTOINCREMENT, order_id TEXT, product_id TEXT, quantity INTEGER DEFAULT 1)").await.ok();

    vil_vwfd::app("examples/606-db-vilorm-ecommerce/vwfd/workflows", 8086)
        .run()
        .await;
}
