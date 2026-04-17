// 608 — Analytics Dashboard (VWFD)
// Business logic identical to standard:
//   POST /events, GET /events/recent, GET /events/by-type,
//   GET /stats/daily, GET /stats/unique-users, GET /stats/summary
use serde_json::{json, Value};
use std::sync::atomic::{AtomicU64, Ordering};

static EVENT_COUNT: AtomicU64 = AtomicU64::new(0);

fn log_event(input: &Value) -> Result<Value, String> {
    let body = &input["body"];
    let id = EVENT_COUNT.fetch_add(1, Ordering::Relaxed) + 1;
    Ok(json!({
        "id": id,
        "event_type": body["event_type"].as_str().unwrap_or("page_view"),
        "user_id": body["user_id"].as_str().unwrap_or("anon"),
        "payload": body.get("payload").cloned().unwrap_or(json!({})),
        "created_at": "2024-01-15T10:30:00Z"
    }))
}

fn recent_events(_input: &Value) -> Result<Value, String> {
    Ok(json!([
        {"id": 1, "event_type": "page_view", "user_id": "u-42", "created_at": "2024-01-15T10:30:00Z"},
        {"id": 2, "event_type": "click", "user_id": "u-42", "created_at": "2024-01-15T10:30:05Z"},
        {"id": 3, "event_type": "purchase", "user_id": "u-17", "created_at": "2024-01-15T10:31:00Z"}
    ]))
}

fn events_by_type(_input: &Value) -> Result<Value, String> {
    Ok(json!([
        {"event_type": "page_view", "count": 1250},
        {"event_type": "click", "count": 890},
        {"event_type": "purchase", "count": 145},
        {"event_type": "signup", "count": 67}
    ]))
}

fn daily_stats(_input: &Value) -> Result<Value, String> {
    Ok(json!([
        {"date": "2024-01-15", "total_events": 2352, "unique_users": 456},
        {"date": "2024-01-14", "total_events": 2100, "unique_users": 412},
        {"date": "2024-01-13", "total_events": 1890, "unique_users": 389}
    ]))
}

fn unique_users(_input: &Value) -> Result<Value, String> {
    Ok(json!({
        "total_unique_users": 1247,
        "last_24h": 456,
        "last_7d": 892,
        "last_30d": 1247
    }))
}

fn stats_summary(_input: &Value) -> Result<Value, String> {
    let total = EVENT_COUNT.load(Ordering::Relaxed);
    Ok(json!({
        "total_events": total,
        "unique_users": 1247,
        "event_types": ["page_view", "click", "purchase", "signup"],
        "events_last_hour": 145
    }))
}

#[tokio::main]
async fn main() {
    vil_vwfd::app("examples/608-db-vilorm-analytics/vwfd/workflows", 8088)
        .native("log_event", log_event)
        .native("recent_events", recent_events)
        .native("events_by_type", events_by_type)
        .native("daily_stats", daily_stats)
        .native("unique_users", unique_users)
        .native("stats_summary", stats_summary)
        .run().await;
}
