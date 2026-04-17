// 506 — VIL Log: Structured Events (VWFD)
// Demonstrates: All 7 log categories with real-world examples
// Standard equivalent: CLI showcasing access/app/ai/db/mq/system/security logs
use serde_json::{json, Value};

fn villog_structured_events(_input: &Value) -> Result<Value, String> {
    Ok(json!({
        "drain": "stdout_pretty",
        "categories": {
            "access_log": {"method": "POST", "path": "/api/orders", "status": 201, "latency_ms": 45.2, "protocol": "HTTP/2"},
            "app_log": [
                {"event": "order.created", "order_id": "ORD-2024-001", "amount_cents": 15000},
                {"event": "payment.captured", "order_id": "ORD-2024-001", "provider": "stripe"},
                {"event": "order.shipped", "order_id": "ORD-2024-001", "carrier": "jne"},
                {"event": "sla.breach", "order_id": "ORD-2024-001", "threshold_hours": 48}
            ],
            "ai_log": {"model": "gpt-4o-mini", "provider": "openai", "tokens_in": 320, "tokens_out": 150, "latency_ms": 890, "cache_hit": false},
            "db_log": [
                {"op": "INSERT", "table": "orders", "engine": "postgres", "latency_ms": 3.2},
                {"op": "SELECT", "table": "products", "engine": "postgres", "latency_ms": 250, "slow_query": true}
            ],
            "mq_log": [
                {"op": "publish", "topic": "order.events", "broker": "kafka", "partition": 3},
                {"op": "dlq", "topic": "order.events.dlq", "reason": "max_retries_exceeded"}
            ],
            "system_log": [
                {"metric": "cpu", "value": 45.2, "unit": "percent", "status": "normal"},
                {"metric": "memory", "value": 82.1, "unit": "percent", "status": "high"}
            ],
            "security_log": [
                {"event": "auth.success", "user_id": "u-42", "method": "oauth2", "ip": "10.0.1.50"},
                {"event": "auth.brute_force", "ip": "203.0.113.5", "attempts": 15, "blocked": true}
            ]
        },
        "total_categories": 7
    }))
}

#[tokio::main]
async fn main() {
    vil_vwfd::app("examples/506-villog-structured-events/vwfd/workflows", 3237)
        .native("villog_structured_events", villog_structured_events)
        .run().await;
}
