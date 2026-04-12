// 009 — SLIK Regulatory Report (Java WASM + Native error handler)
use serde_json::{json, Value};
fn handle_slik_error(input: &Value) -> Result<Value, String> {
    let error = input["error"].as_str().unwrap_or("unknown");
    Ok(json!({"handled": true, "original_error": error, "action": "logged_and_skipped"}))
}
#[tokio::main]
async fn main() {
    vil_vwfd::app("examples/009-basic-credit-regulatory-slik/vwfd/workflows", 3109)
        .wasm("format_slik_report", "examples/009-basic-credit-regulatory-slik/vwfd/wasm/java/SlikReportFormatter.class")
        .native("handle_slik_error", handle_slik_error)
        .run().await;
}
