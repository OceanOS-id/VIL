// 029 — VIL Handler Endpoint Demo
use serde_json::json;

#[tokio::main]
async fn main() {
    vil_vwfd::app("examples/029-basic-vil-handler-endpoint/vwfd/workflows", 8080)
        .native("plain_handler", |_| {
            Ok(json!({
                "message": "Hello from plain handler",
                "style": "plain"
            }))
        })
        .native("handled_handler", |_| {
            let req_id = format!("req-{}", std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_millis());
            Ok(json!({
                "message": "Hello from vil_handler macro",
                "request_id": req_id,
                "style": "handled"
            }))
        })
        .native("endpoint_handler", |input| {
            let body = input.get("body").cloned().unwrap_or(json!({}));
            let value = body.get("value").and_then(|v| v.as_i64()).unwrap_or(0);
            let result = value * value;
            Ok(json!({
                "result": result,
                "input": value,
                "style": "endpoint"
            }))
        })
        .run()
        .await;
}
