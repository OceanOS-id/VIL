// 004 — Task CRUD (Pure VWFD — multiple workflows, VilQuery inline, zero NativeCode)
//
// 6 workflow YAMLs, 1 per route:
//   POST   /api/tasks/tasks       → create-task.yaml (VilQuery insert)
//   GET    /api/tasks/tasks       → list-tasks.yaml (VilQuery select)
//   GET    /api/tasks/tasks/:id   → get-task.yaml (VilQuery find_one)
//   PUT    /api/tasks/tasks/:id   → update-task.yaml (VilQuery update)
//   DELETE /api/tasks/tasks/:id   → delete-task.yaml (VilQuery delete)
//   GET    /api/tasks/tasks/stats → task-stats.yaml (VilQuery count)

#[tokio::main]
async fn main() {
    // Set SQLite DB if not already configured
    if std::env::var("VIL_DATABASE_URL").is_err() {
        let db_path = format!("{}/tasks_vwfd.db", std::env::temp_dir().display());
        std::env::set_var("VIL_DATABASE_URL", format!("sqlite:{}?mode=rwc", db_path));
    }
    // Init table at startup (before any workflow runs)
    {
        let url = std::env::var("VIL_DATABASE_URL").unwrap();
        let pool = vil_db_sqlx::SqlxPool::connect("init", vil_db_sqlx::SqlxConfig::sqlite(&url))
            .await.expect("SQLite connect");
        pool.execute_raw(
            "CREATE TABLE IF NOT EXISTS tasks (
                id TEXT PRIMARY KEY, title TEXT NOT NULL, description TEXT DEFAULT '',
                done INTEGER DEFAULT 0,
                created_at TEXT DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ','now')),
                updated_at TEXT DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ','now'))
            )"
        ).await.expect("init table");
    }
    // VilQuery inline workflows — all DB logic in YAML, zero Rust handlers
    vil_vwfd::app("examples/004-basic-rest-crud/vwfd/workflows", 8080)
        .run()
        .await;
}
