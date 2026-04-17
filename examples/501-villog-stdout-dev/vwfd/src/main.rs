// 501 — VIL Log: Stdout Dev Mode (VWFD)
// Demonstrates: Colored structured logging to stdout with Pretty format
// Standard equivalent: CLI that emits app_log!, access_log!, ai_log! to StdoutDrain
use serde_json::{json, Value};

fn villog_stdout_demo(_input: &Value) -> Result<Value, String> {
    Ok(json!({
        "drain": "stdout",
        "format": "pretty",
        "sample_logs": [
            {"level": "INFO", "category": "app", "message": "Server started on :8080", "fields": {"service": "api-gateway", "version": "1.2.3"}},
            {"level": "INFO", "category": "access", "message": "HTTP/2 GET /api/health → 200", "fields": {"method": "GET", "path": "/api/health", "status": 200, "latency_ms": 1.2}},
            {"level": "DEBUG", "category": "ai", "message": "LLM chat completion", "fields": {"model": "gpt-4", "tokens_in": 150, "tokens_out": 89, "latency_ms": 340}}
        ],
        "features": ["colored_output", "human_readable", "structured_fields", "category_prefix"]
    }))
}

#[tokio::main]
async fn main() {
    vil_vwfd::app("examples/501-villog-stdout-dev/vwfd/workflows", 3232)
        .native("villog_stdout_demo", villog_stdout_demo)
        .run().await;
}
