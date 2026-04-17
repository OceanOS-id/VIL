// 503 — VIL Log: Multi-Drain Fan-Out (VWFD)
// Demonstrates: MultiDrain combining stdout (compact) + file (10MB rotation)
// Standard equivalent: CLI emitting app_log, access_log, mq_log to both drains
use serde_json::{json, Value};

fn villog_multi_drain(_input: &Value) -> Result<Value, String> {
    Ok(json!({
        "drain": "multi",
        "targets": [
            {"type": "stdout", "format": "compact"},
            {"type": "file", "path": "./logs/app.log", "rotation": "size", "max_size_mb": 10}
        ],
        "sample_events": {"app_log": 4, "access_log": 5, "mq_log": 2},
        "features": ["fan_out", "independent_formats", "size_rotation", "dual_output"]
    }))
}

#[tokio::main]
async fn main() {
    vil_vwfd::app("examples/503-villog-multi-drain/vwfd/workflows", 3234)
        .native("villog_multi_drain", villog_multi_drain)
        .run().await;
}
