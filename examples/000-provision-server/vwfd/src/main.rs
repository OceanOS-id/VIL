// 000-provision-server — VIL provisionable server for testsuite
//
// Starts empty server with provision API enabled.
// Workflows uploaded at runtime via POST /api/admin/upload.
//
// Env vars:
//   PORT            — HTTP port (default: 8080)
//   ADMIN_KEY       — API key for admin endpoints (default: none)
//   WORKFLOWS_DIR   — workflow persistence directory (default: /tmp/vil-provision)

use vil_vwfd::StateStore;

#[tokio::main]
async fn main() {
    let port: u16 = std::env::var("PORT")
        .ok().and_then(|p| p.parse().ok()).unwrap_or(8080);
    let workflows_dir = std::env::var("WORKFLOWS_DIR")
        .unwrap_or_else(|_| "/tmp/vil-provision".into());
    let _ = std::fs::create_dir_all(&workflows_dir);

    let mut app = vil_vwfd::app(&workflows_dir, port)
        .provision(true)
        .state_store(StateStore::H2InMemory);

    if let Ok(key) = std::env::var("ADMIN_KEY") {
        app = app.provision_key(key);
    }

    app.run().await;
}
