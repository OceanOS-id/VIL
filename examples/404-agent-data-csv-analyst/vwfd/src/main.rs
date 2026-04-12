// 404 — CSV Data Analyst (C WASM stats + Native chart)
use serde_json::{json, Value};
fn chart_generator(input: &Value) -> Result<Value, String> {
    let labels = input["labels"].as_array().map(|a| a.to_vec()).unwrap_or_default();
    let data = input["data"].as_array().map(|a| a.to_vec()).unwrap_or_default();
    Ok(json!({"type": "bar", "data": {"labels": labels, "datasets": [{"data": data}]}}))
}
#[tokio::main]
async fn main() {
    vil_vwfd::app("examples/404-agent-data-csv-analyst/vwfd/workflows", 3123)
        .wasm("csv_stats_engine", "examples/404-agent-data-csv-analyst/vwfd/wasm/c/csv_stats.wasm")
        .native("chart_generator", chart_generator)
        .run().await;
}
