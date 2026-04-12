// 104-pipeline-diamond-topology — VWFD mode
use serde_json::{json, Value};

#[tokio::main]
async fn main() {
    vil_vwfd::app("examples/104-pipeline-diamond-topology/vwfd/workflows", 3206)
        .native("diamond_dual_view", |input| {
            // 104-pipeline-diamond-topology: diamond_dual_view
            Ok(serde_json::json!({"_handler": "diamond_dual_view", "input_keys": input.as_object().map(|o| o.keys().collect::<Vec<_>>())}))
        })
        .run()
        .await;
}
