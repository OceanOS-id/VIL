// 905 — Notification Hub (Workflow Pattern)
// Demonstrates: email, webhook_out, template, mask, id_gen in VIL Expression
#[tokio::main]
async fn main() {
    vil_vwfd::app("examples/905-faas-notification-hub/vwfd/workflows", 8080)
        .run()
        .await;
}
