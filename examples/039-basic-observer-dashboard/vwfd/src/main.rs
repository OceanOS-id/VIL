// 039-basic-observer-dashboard — VWFD mode
use serde_json::json;

#[tokio::main]
async fn main() {
    vil_vwfd::app("examples/039-basic-observer-dashboard/vwfd/workflows", 8080)
        .observer(true)
        .native("hello_handler", |_| {
            Ok(json!({"message": "Hello from Observer Dashboard!", "version": "1.0"}))
        })
        .native("echo_handler", |input| {
            let body = &input["body"];
            let bytes = serde_json::to_vec(body).unwrap_or_default().len();
            Ok(json!({"echo": body, "received": true, "received_bytes": bytes}))
        })
        .run()
        .await;
}
