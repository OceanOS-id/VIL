// 505 — VIL Log: Tracing Bridge (VWFD)
// Demonstrates: VilTracingLayer bridging third-party tracing → VIL ring
// Standard equivalent: CLI with tracing spans routed through VIL drain
use serde_json::{json, Value};

fn villog_tracing_bridge(_input: &Value) -> Result<Value, String> {
    Ok(json!({
        "bridge": "VilTracingLayer",
        "direction": "tracing → vil_log ring",
        "drain": "stdout_compact",
        "simulated_spans": [
            {"name": "process_request", "level": "INFO", "fields": {"request_id": "req-abc123", "method": "POST", "path": "/api/orders"}},
            {"name": "authenticate", "level": "DEBUG", "fields": {"user_id": "u-42", "method": "bearer_token", "valid": true}},
            {"name": "database_error", "level": "ERROR", "fields": {"query": "INSERT INTO orders", "error": "connection refused", "retry": 3}}
        ],
        "features": ["bidirectional", "tracing_subscriber", "span_to_event", "zero_copy_bridge"]
    }))
}

#[tokio::main]
async fn main() {
    vil_vwfd::app("examples/505-villog-tracing-bridge/vwfd/workflows", 3236)
        .native("villog_tracing_bridge", villog_tracing_bridge)
        .run().await;
}
