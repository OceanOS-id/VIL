// 504-villog-benchmark-comparison — VWFD mode
use serde_json::{json, Value};

#[tokio::main]
async fn main() {
    vil_vwfd::app("examples/504-villog-benchmark-comparison/vwfd/workflows", 3235)
        .native("villog_benchmark", |input| {
            // 504-villog-benchmark-comparison: villog_benchmark
            Ok(serde_json::json!({"_handler": "villog_benchmark", "input_keys": input.as_object().map(|o| o.keys().collect::<Vec<_>>())}))
        })
        .run()
        .await;
}
