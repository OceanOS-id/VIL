// 013-basic-nats-worker — NATS pub/sub with JetStream (VWFD)
//
// Endpoints:
//   GET  /api/nats/config    → NATS config info
//   POST /api/nats/publish   → publish message (returns "published" + "subject")
//   GET  /api/nats/jetstream → JetStream info
//   GET  /api/nats/kv        → KV store info

use serde_json::{json, Value};

fn nats_config(_input: &Value) -> Result<Value, String> {
    Ok(json!({
        "nats": {
            "url": "nats://localhost:19222",
            "cluster": "vil-test",
            "max_reconnects": 60
        },
        "jetstream": { "enabled": true },
        "kv_buckets": ["cache", "sessions"]
    }))
}

fn nats_publish(input: &Value) -> Result<Value, String> {
    let subject = input.get("body")
        .and_then(|b| b["subject"].as_str())
        .unwrap_or("events.default");
    Ok(json!({
        "published": true,
        "subject": subject,
        "sequence": 1,
        "stream": "EVENTS"
    }))
}

fn nats_jetstream(_input: &Value) -> Result<Value, String> {
    Ok(json!({
        "streams": [
            {"name": "EVENTS", "subjects": ["events.>"], "messages": 0, "bytes": 0}
        ],
        "consumers": [],
        "status": "active"
    }))
}

fn nats_kv(_input: &Value) -> Result<Value, String> {
    Ok(json!({
        "buckets": [
            {"name": "cache", "keys": 0, "bytes": 0},
            {"name": "sessions", "keys": 0, "bytes": 0}
        ],
        "status": "active"
    }))
}

#[tokio::main]
async fn main() {
    vil_vwfd::app("examples/013-basic-nats-worker/vwfd/workflows", 8080)
        .native("nats_config", nats_config)
        .native("nats_publish", nats_publish)
        .native("nats_jetstream", nats_jetstream)
        .native("nats_kv", nats_kv)
        .run()
        .await;
}
