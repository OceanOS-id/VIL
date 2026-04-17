// 605 — Blog Platform (Pure VilQuery Workflow — no NativeCode DB access)
//
// ALL DB operations via VilQuery Connector in workflow YAML:
//   GET  /authors     → VilQuery select
//   POST /authors     → VilQuery insert
//   GET  /posts       → VilQuery select (JOIN)
//   POST /posts       → VilQuery insert
//   GET  /posts/:id   → VilQuery select
//   PUT  /posts/:id   → VilQuery update
//   DELETE /posts/:id → VilQuery delete
//   GET  /stats       → VilQuery count
//   POST /tags        → VilQuery insert (on conflict nothing)
//
// No NativeCode handlers — 100% workflow-driven DB.
// Compatible with provision mode (no .so needed for DB ops).

#[tokio::main]
async fn main() {
    // Init SQLite DB + tables
    if std::env::var("VIL_DATABASE_URL").is_err() {
        let p = format!("{}/blog_vwfd.db", std::env::temp_dir().display());
        std::env::set_var("VIL_DATABASE_URL", format!("sqlite:{}?mode=rwc", p));
    }
    let url = std::env::var("VIL_DATABASE_URL").unwrap();
    let db = vil_db_sqlx::SqlxPool::connect("blog", vil_db_sqlx::SqlxConfig::sqlite(&url))
        .await.expect("db connect");

    db.execute_raw("CREATE TABLE IF NOT EXISTS authors (id TEXT PRIMARY KEY, name TEXT NOT NULL, bio TEXT, created_at TEXT DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ','now')))").await.ok();
    db.execute_raw("CREATE TABLE IF NOT EXISTS posts (id TEXT PRIMARY KEY, author_id TEXT, title TEXT NOT NULL, content TEXT DEFAULT '', status TEXT DEFAULT 'draft', views INTEGER DEFAULT 0, created_at TEXT DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ','now')))").await.ok();
    db.execute_raw("CREATE TABLE IF NOT EXISTS tags (id INTEGER PRIMARY KEY AUTOINCREMENT, name TEXT UNIQUE NOT NULL)").await.ok();

    vil_vwfd::app("examples/605-db-vilorm-crud/vwfd/workflows", 8080)
        .run()
        .await;
}
