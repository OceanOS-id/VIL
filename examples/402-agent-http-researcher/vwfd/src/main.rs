use serde_json::{json, Value};
fn research_tool_executor(input: &Value) -> Result<Value, String> {
    let tool = input["tool"].as_str().unwrap_or("");
    match tool {
        "fetch_products" => Ok(json!({"products": [{"name": "Widget", "price": 29.99}, {"name": "Gadget", "price": 49.99}]})),
        "calculator" => {
            let values = input["values"].as_array().map(|a| a.iter().filter_map(|v| v.as_f64()).collect::<Vec<_>>()).unwrap_or_default();
            let sum: f64 = values.iter().sum();
            let avg = if values.is_empty() { 0.0 } else { sum / values.len() as f64 };
            Ok(json!({"sum": sum, "avg": avg, "count": values.len()}))
        }
        _ => Err(format!("unknown tool: {}", tool)),
    }
}
#[tokio::main]
async fn main() {
    vil_vwfd::app("examples/402-agent-http-researcher/vwfd/workflows", 8080)
        .native("research_tool_executor", research_tool_executor).run().await;
}
