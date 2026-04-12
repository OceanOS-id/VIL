// 505-villog-tracing-bridge — VWFD mode
use serde_json::{json, Value};

#[tokio::main]
async fn main() {
    vil_vwfd::app("examples/505-villog-tracing-bridge/vwfd/workflows", 3236)
        .native("villog_tracing_bridge", |input| {
            // 505-villog-tracing-bridge: villog_tracing_bridge
            Ok(serde_json::json!({"_handler": "villog_tracing_bridge", "input_keys": input.as_object().map(|o| o.keys().collect::<Vec<_>>())}))
        })
        .run()
        .await;
}
