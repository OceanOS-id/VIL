// VIL Provisionable Server
//
// Starts an empty server with the admin API enabled.
// Workflows are uploaded at runtime via POST /api/admin/upload.
//
// Env vars:
//   PORT          — HTTP listen port       (default: 8080)
//   ADMIN_KEY     — API key for admin API  (default: none / open)
//   WORKFLOWS_DIR — persistence directory  (default: /tmp/vil-workflows)

use vil_vwfd::StateStore;

#[tokio::main]
async fn main() {
    let port: u16 = std::env::var("PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(3080);

    let workflows_dir = std::env::var("WORKFLOWS_DIR")
        .unwrap_or_else(|_| "/tmp/vil-workflows".into());

    let _ = std::fs::create_dir_all(&workflows_dir);

    let mut app = vil_vwfd::app(&workflows_dir, port)
        .provision(true)
        .state_store(StateStore::H2InMemory);

    if let Ok(key) = std::env::var("ADMIN_KEY") {
        if !key.is_empty() {
            app = app.provision_key(key);
        }
    }

    app.run().await;
}
