// 903 — Secure API Gateway (Workflow Pattern)
// Demonstrates: crypto, jwt, webhook_out, regex, template in V-CEL
#[tokio::main]
async fn main() {
    vil_vwfd::app("examples/903-faas-secure-api/vwfd/workflows", 8080)
        .run()
        .await;
}
