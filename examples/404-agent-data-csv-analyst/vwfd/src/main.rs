// 404 — CSV Data Analyst (VWFD)
// Business logic identical to standard:
//   POST /api/csv-analyze — parse CSV → stats (mean/median/std_dev/growth) →
//   Chart.js chart data → LLM narrative analysis
use serde_json::{json, Value};

fn chart_generator(input: &Value) -> Result<Value, String> {
    let stats = &input["stats"];
    let columns: Vec<String> = stats.as_object()
        .map(|o| o.keys().cloned().collect())
        .unwrap_or_default();

    let chart_type = input["chart_preference"].as_str().unwrap_or("bar");
    let labels: Vec<String> = columns.iter().cloned().collect();
    let data: Vec<f64> = columns.iter()
        .filter_map(|c| stats[c]["mean"].as_f64())
        .collect();

    Ok(json!({
        "chart_type": chart_type,
        "title": "Data Analysis",
        "labels": labels,
        "datasets": [{"label": "mean", "data": data}]
    }))
}

#[tokio::main]
async fn main() {
    vil_vwfd::app("examples/404-agent-data-csv-analyst/vwfd/workflows", 3123)
        .wasm("csv_stats_engine", "examples/404-agent-data-csv-analyst/vwfd/wasm/c/csv_stats.wasm")
        .native("chart_generator", chart_generator)
        .run().await;
}
