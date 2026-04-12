// 044 — GraphQL Subscriptions (Hybrid: Sidecar Node.js for notification building, NativeCode for stats)
use serde_json::json;

#[tokio::main]
async fn main() {
    vil_vwfd::app("examples/044-basic-graphql-subscriptions/vwfd/workflows", 8080)
        // Notification publish — Sidecar Node.js (external Node runtime)
        .sidecar("notification_builder", "node examples/044-basic-graphql-subscriptions/vwfd/sidecar/nodejs/notification_builder.js")
        // Stats — NativeCode (simple counter)
        .native("stats_handler", |_| {
            Ok(json!({"active_subscribers": 0, "total_published": 0}))
        })
        .run()
        .await;
}
