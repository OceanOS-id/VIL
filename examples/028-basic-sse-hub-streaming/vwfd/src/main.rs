// 028 — SSE Hub Streaming (Live Auction)
use serde_json::json;

#[tokio::main]
async fn main() {
    vil_vwfd::app("examples/028-basic-sse-hub-streaming/vwfd/workflows", 8080)
        .native("sse_stats_handler", |_| {
            Ok(json!({
                "connected_clients": 0,
                "total_events": 0,
                "current_item": "lot-001"
            }))
        })
        .native("sse_publish_handler", |input| {
            let body = input.get("body").cloned().unwrap_or(json!({}));
            let message = body.get("message").and_then(|v| v.as_str()).unwrap_or("event");
            Ok(json!({
                "published": true,
                "message": message
            }))
        })
        .run()
        .await;
}
