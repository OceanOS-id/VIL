// 101b-multi-pipeline-benchmark — VWFD mode
use serde_json::{json, Value};

#[tokio::main]
async fn main() {
    vil_vwfd::app("examples/101b-multi-pipeline-benchmark/vwfd/workflows", 3201)
        .native("pipeline_benchmark", |input| {
            // 101b-multi-pipeline-benchmark: pipeline_benchmark
            Ok(serde_json::json!({"_handler": "pipeline_benchmark", "input_keys": input.as_object().map(|o| o.keys().collect::<Vec<_>>())}))
        })
        .run()
        .await;
}
