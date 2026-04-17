// 706 — FinTech Transaction Event Bus (VWFD)
// Business logic identical to standard:
//   POST /api/events/publish → PublishResponse { status, event_id, topic, queue_depth }
//   GET /api/events/consume → ConsumeResponse { status, event, remaining, consumer_group }
//   GET /api/events/stats → StatsResponse { total_published, total_consumed, queue_depth, topics, note }
use serde_json::{json, Value};
use std::sync::atomic::{AtomicU64, Ordering};

static PUBLISHED: AtomicU64 = AtomicU64::new(0);
static CONSUMED: AtomicU64 = AtomicU64::new(0);

fn consume_event(_input: &Value) -> Result<Value, String> {
    let consumed = CONSUMED.fetch_add(1, Ordering::Relaxed);
    let published = PUBLISHED.load(Ordering::Relaxed);
    let remaining = if published > consumed { published - consumed } else { 0 };
    Ok(json!({
        "status": if remaining > 0 { "consumed" } else { "empty" },
        "event": if remaining > 0 {
            json!({
                "event_id": format!("EVT-{:06}", consumed + 1),
                "event_type": "purchase",
                "transaction_id": format!("TXN-{:06}", consumed + 1),
                "amount": 59.99,
                "currency": "IDR",
                "from_account": "ACC-001",
                "to_account": "ACC-002",
                "published_at": "2024-01-15T10:30:00Z",
                "status": "completed"
            })
        } else {
            json!(null)
        },
        "remaining": remaining,
        "consumer_group": "fintech-processor-01"
    }))
}

fn event_stats(_input: &Value) -> Result<Value, String> {
    let published = PUBLISHED.load(Ordering::Relaxed);
    let consumed = CONSUMED.load(Ordering::Relaxed);
    Ok(json!({
        "total_published": published,
        "total_consumed": consumed,
        "queue_depth": if published > consumed { published - consumed } else { 0 },
        "topics": [
            {"topic": "persistent://fintech/txn/purchase", "published": published / 2 + 1, "description": "Purchase transactions"},
            {"topic": "persistent://fintech/txn/transfer", "published": published / 3 + 1, "description": "Fund transfers"},
            {"topic": "persistent://fintech/txn/refund", "published": published / 6, "description": "Refund events"}
        ],
        "note": "Pulsar persistent topics with tenant=fintech, namespace=txn"
    }))
}

#[tokio::main]
async fn main() {
    vil_vwfd::app("examples/706-mq-pulsar-messaging/vwfd/workflows", 8080)
        .native("consume_event", consume_event)
        .native("event_stats", event_stats)
        .run().await;
}
