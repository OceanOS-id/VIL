// 108-workflow-dag-scheduler — VWFD mode
use serde_json::{json, Value};

#[tokio::main]
async fn main() {
    vil_vwfd::app("examples/108-workflow-dag-scheduler/vwfd/workflows", 8080)
        .native("dag_etl_scheduler", |input| {
            // 108-workflow-dag-scheduler: dag_etl_scheduler
            Ok(serde_json::json!({"_handler": "dag_etl_scheduler", "input_keys": input.as_object().map(|o| o.keys().collect::<Vec<_>>())}))
        })
        .run()
        .await;
}
