// 506-villog-structured-events — VWFD mode
use serde_json::{json, Value};

#[tokio::main]
async fn main() {
    vil_vwfd::app("examples/506-villog-structured-events/vwfd/workflows", 3237)
        .native("villog_structured_events", |input| {
            // 506-villog-structured-events: villog_structured_events
            Ok(serde_json::json!({"_handler": "villog_structured_events", "input_keys": input.as_object().map(|o| o.keys().collect::<Vec<_>>())}))
        })
        .run()
        .await;
}
