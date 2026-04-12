// 105-pipeline-multi-workflow — VWFD mode
use serde_json::{json, Value};

#[tokio::main]
async fn main() {
    vil_vwfd::app("examples/105-pipeline-multi-workflow/vwfd/workflows", 3207)
        .native("multi_workflow_concurrent", |input| {
            // 105-pipeline-multi-workflow: multi_workflow_concurrent
            Ok(serde_json::json!({"_handler": "multi_workflow_concurrent", "input_keys": input.as_object().map(|o| o.keys().collect::<Vec<_>>())}))
        })
        .run()
        .await;
}
