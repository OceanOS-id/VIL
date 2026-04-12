// 106-pipeline-sse-standard-dialect — VWFD mode
use serde_json::{json, Value};

#[tokio::main]
async fn main() {
    vil_vwfd::app("examples/106-pipeline-sse-standard-dialect/vwfd/workflows", 3208)
        .native("sse_standard_stream", |input| {
            // 106-pipeline-sse-standard-dialect: sse_standard_stream
            Ok(serde_json::json!({"_handler": "sse_standard_stream", "input_keys": input.as_object().map(|o| o.keys().collect::<Vec<_>>())}))
        })
        .run()
        .await;
}
