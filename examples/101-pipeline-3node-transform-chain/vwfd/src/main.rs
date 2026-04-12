// 101-pipeline-3node-transform-chain — VWFD mode
use serde_json::{json, Value};

#[tokio::main]
async fn main() {
    vil_vwfd::app("examples/101-pipeline-3node-transform-chain/vwfd/workflows", 3203)
        .native("etl_transform_chain", |input| {
            // 101-pipeline-3node-transform-chain: etl_transform_chain
            Ok(serde_json::json!({"_handler": "etl_transform_chain", "input_keys": input.as_object().map(|o| o.keys().collect::<Vec<_>>())}))
        })
        .run()
        .await;
}
