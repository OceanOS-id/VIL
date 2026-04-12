// 101c-vilapp-multi-pipeline-benchmark — VWFD mode
use serde_json::{json, Value};

#[tokio::main]
async fn main() {
    vil_vwfd::app("examples/101c-vilapp-multi-pipeline-benchmark/vwfd/workflows", 3202)
        .native("vilapp_pipeline_benchmark", |input| {
            // 101c-vilapp-multi-pipeline-benchmark: vilapp_pipeline_benchmark
            Ok(serde_json::json!({"_handler": "vilapp_pipeline_benchmark", "input_keys": input.as_object().map(|o| o.keys().collect::<Vec<_>>())}))
        })
        .run()
        .await;
}
