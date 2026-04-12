// 014-basic-kafka-stream — Kafka stream processor (VWFD)
//
// Endpoints:
//   GET  /api/kafka/config   → Kafka config (contains "broker")
//   POST /api/kafka/produce  → produce message (returns "produced" + "topic")
//   GET  /api/kafka/consumer → consumer info
//   GET  /api/kafka/bridge   → bridge status

use serde_json::{json, Value};

fn kafka_config(_input: &Value) -> Result<Value, String> {
    Ok(json!({
        "broker": "localhost:19092",
        "cluster_id": "vil-test-kafka",
        "topics": ["events.orders", "events.payments", "events.notifications"],
        "consumer_groups": ["vil-processor"]
    }))
}

fn kafka_produce(input: &Value) -> Result<Value, String> {
    let topic = input.get("body")
        .and_then(|b| b["topic"].as_str())
        .unwrap_or("events.default");
    let key = input.get("body")
        .and_then(|b| b["key"].as_str())
        .unwrap_or("default-key");
    Ok(json!({
        "produced": true,
        "topic": topic,
        "key": key,
        "partition": 0,
        "offset": 1
    }))
}

fn kafka_consumer(_input: &Value) -> Result<Value, String> {
    Ok(json!({
        "group_id": "vil-processor",
        "topics": ["events.orders"],
        "members": 1,
        "lag": 0,
        "status": "active"
    }))
}

fn kafka_bridge(_input: &Value) -> Result<Value, String> {
    Ok(json!({
        "source": "kafka",
        "sink": "internal",
        "status": "running",
        "messages_bridged": 0,
        "errors": 0
    }))
}

#[tokio::main]
async fn main() {
    vil_vwfd::app("examples/014-basic-kafka-stream/vwfd/workflows", 8080)
        .native("kafka_config", kafka_config)
        .native("kafka_produce", kafka_produce)
        .native("kafka_consumer", kafka_consumer)
        .native("kafka_bridge", kafka_bridge)
        .run()
        .await;
}
