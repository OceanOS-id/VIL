// 509-villog-phase1-integration — VWFD mode
use serde_json::{json, Value};

#[tokio::main]
async fn main() {
    vil_vwfd::app("examples/509-villog-phase1-integration/vwfd/workflows", 3240)
        .native("villog_phase1_integration", |input| {
            // 509-villog-phase1-integration: villog_phase1_integration
            Ok(serde_json::json!({"_handler": "villog_phase1_integration", "input_keys": input.as_object().map(|o| o.keys().collect::<Vec<_>>())}))
        })
        .run()
        .await;
}
