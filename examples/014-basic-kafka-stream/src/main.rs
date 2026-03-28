// ╔════════════════════════════════════════════════════════════╗
// ║  014 — Transaction Event Stream (Kafka)                   ║
// ╠════════════════════════════════════════════════════════════╣
// ║  Pattern:  VX_APP                                           ║
// ║  Token:    N/A (HTTP server)                                ║
// ║  Features: ShmSlice, ServiceCtx, VilResponse                ║
// ║  Domain:   Financial transaction events streamed through    ║
// ║            Kafka topics for audit trail and compliance      ║
// ╚════════════════════════════════════════════════════════════╝
// basic-usage-kafka-stream — Kafka Consumer → Process → Produce (VX Process-Oriented)
// =============================================================================
//
// BUSINESS CONTEXT:
//   Financial transaction audit trail. Every payment, refund, and transfer is
//   published to Kafka with key-based partitioning (customer ID) to guarantee
//   per-customer ordering. The "events.orders" topic feeds the compliance
//   team's audit dashboard, "events.payments" drives the reconciliation
//   engine, and "events.notifications" triggers customer receipts.
//   Kafka's immutable log ensures regulatory auditability (PCI-DSS, SOX).
//
// Demonstrates vil_mq_kafka integration for Kafka-based stream processing
// using the VX Process-Oriented architecture (VilApp + ServiceProcess).
// The Kafka producer and consumer use in-memory implementations, so this example runs
// without a real Kafka cluster.
//
// Features demonstrated:
//   - KafkaConfig — broker connection, SASL auth, consumer groups
//   - KafkaProducer — publish messages with optional key-based partitioning
//   - KafkaConsumer — consumer group pattern with message injection
//   - KafkaBridge — Kafka → Tri-Lane SHM zero-copy bridge
//   - Stream processing pipeline pattern
//
// Routes:
//   GET  /                    → overview
//   GET  /api/kafka/config    → Kafka broker configuration and status
//   POST /api/kafka/produce   → produce a message to a topic
//   GET  /api/kafka/consumer  → consumer group configuration info
//   GET  /api/kafka/bridge    → Kafka → Tri-Lane bridge status
//
// Built-in endpoints (auto-provided by VilServer):
//   GET  /health, /ready, /metrics, /info
//
// Run:
//   cargo run -p basic-usage-kafka-stream
//
// Test:
//   curl http://localhost:8080/
//   curl http://localhost:8080/api/kafka/config
//   curl -X POST http://localhost:8080/api/kafka/produce \
//     -H 'Content-Type: application/json' \
//     -d '{"topic":"events.orders","key":"order-42","payload":{"action":"created","amount":199.99}}'
//   curl http://localhost:8080/api/kafka/consumer
//   curl http://localhost:8080/api/kafka/bridge
// =============================================================================

use vil_server::axum::extract::Extension;
use vil_server::prelude::*;

use vil_mq_kafka::{KafkaBridge, KafkaConfig, KafkaProducer};

use std::sync::Arc;

// ---------------------------------------------------------------------------
// Shared state
// ---------------------------------------------------------------------------

#[derive(Clone)]
struct KafkaState {
    producer: Arc<KafkaProducer>,
    bridge: Arc<KafkaBridge>,
    config: KafkaConfig,
    consumer_config: KafkaConfig,
}

// ---------------------------------------------------------------------------
// Response types
// ---------------------------------------------------------------------------

#[derive(Clone, Debug, Serialize, Deserialize, VilModel)]
struct TopicInfo {
    name: String,
    partitions: u32,
    replication_factor: u32,
    description: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, VilModel)]
struct BrokerInfo {
    brokers: String,
    acks: String,
    timeout_ms: u64,
    security_protocol: Option<String>,
    sasl_mechanism: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, VilModel)]
struct ProducerInfo {
    messages_sent: u64,
    errors: u64,
    status: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, VilModel)]
struct KafkaConfigResponse {
    broker: BrokerInfo,
    producer: ProducerInfo,
    topics: Vec<TopicInfo>,
    note: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, VilModel)]
struct ProduceResponse {
    status: String,
    topic: String,
    key: Option<String>,
    payload_size: usize,
    total_produced: u64,
    partitioning: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, VilModel)]
struct ConsumerGroupInfo {
    group_id: Option<String>,
    brokers: String,
    topic: Option<String>,
    acks: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, VilModel)]
struct PipelineStep {
    step: u32,
    name: String,
    description: String,
    code_pattern: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, VilModel)]
struct StreamProcessingPipeline {
    description: String,
    steps: Vec<PipelineStep>,
}

#[derive(Clone, Debug, Serialize, Deserialize, VilModel)]
struct ConsumerInfoResponse {
    consumer_group: ConsumerGroupInfo,
    stream_processing_pipeline: StreamProcessingPipeline,
    consumer_api: std::collections::HashMap<String, String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, VilModel)]
struct TriLaneLane {
    name: String,
    description: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, VilModel)]
struct TriLaneArchitecture {
    description: String,
    lanes: Vec<TriLaneLane>,
    flow: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, VilModel)]
struct KafkaBridgeInfo {
    target_service: String,
    messages_bridged: u64,
    status: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, VilModel)]
struct BridgeStatusResponse {
    bridge: KafkaBridgeInfo,
    tri_lane_architecture: TriLaneArchitecture,
    api: std::collections::HashMap<String, String>,
}

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

/// GET / — overview of the Kafka stream example.
async fn index() -> &'static str {
    "VIL Kafka Stream Example\n\
     ==============================\n\n\
     Demonstrates Kafka consumer → process → produce patterns.\n\n\
     Endpoints:\n\
     - GET  /api/kafka/config    — broker configuration and connection status\n\
     - POST /api/kafka/produce   — produce a message to a Kafka topic\n\
     - GET  /api/kafka/consumer  — consumer group configuration info\n\
     - GET  /api/kafka/bridge    — Kafka → Tri-Lane SHM bridge status\n\n\
     Built-in:\n\
     - GET  /health, /ready, /metrics, /info\n"
}

/// GET /api/kafka/config — KafkaConfig pattern with broker and auth fields.
async fn kafka_config(ctx: ServiceCtx) -> VilResponse<KafkaConfigResponse> {
    let state = ctx.state::<KafkaState>().expect("state type mismatch");
    VilResponse::ok(KafkaConfigResponse {
        broker: BrokerInfo {
            brokers: state.config.brokers.clone(),
            acks: state.config.acks.clone(),
            timeout_ms: state.config.timeout_ms,
            security_protocol: state.config.security_protocol.clone(),
            sasl_mechanism: state.config.sasl_mechanism.clone(),
        },
        producer: ProducerInfo {
            messages_sent: state.producer.messages_sent(),
            errors: state.producer.errors(),
            status: "ready".into(),
        },
        // Topic topology: partition count reflects expected throughput.
        // orders (6 partitions) — highest volume, supports 6 parallel consumers
        // payments (3 partitions) — lower volume, must match order processing rate
        // notifications (3 partitions, RF=2) — non-critical, lower replication
        topics: vec![
            TopicInfo {
                name: "events.orders".into(),
                partitions: 6,
                replication_factor: 3,
                description: "Order lifecycle events".into(),
            },
            TopicInfo {
                name: "events.payments".into(),
                partitions: 3,
                replication_factor: 3,
                description: "Payment processing events".into(),
            },
            TopicInfo {
                name: "events.notifications".into(),
                partitions: 3,
                replication_factor: 2,
                description: "User notification events".into(),
            },
        ],
        note: "Stub mode — no real Kafka broker connected. Messages are processed in-memory."
            .into(),
    })
}

/// Request body for producing a message.
#[derive(Clone, Debug, Serialize, Deserialize, VilModel)]
struct ProduceRequest {
    topic: String,
    #[serde(default)]
    key: Option<String>,
    payload: serde_json::Value,
}

/// POST /api/kafka/produce — produce a message to a Kafka topic.
async fn kafka_produce(
    ctx: ServiceCtx,
    body: ShmSlice,
) -> HandlerResult<VilResponse<ProduceResponse>> {
    let state = ctx.state::<KafkaState>().expect("state type mismatch");
    let req: ProduceRequest = body.json().expect("invalid JSON body");
    let payload_bytes = serde_json::to_vec(&req.payload)
        .map_err(|e| VilError::bad_request(format!("Invalid payload: {}", e)))?;

    // Produce with or without key.
    // Key-based partitioning ensures all events for the same customer/order
    // land on the same partition — critical for maintaining event ordering
    // in the financial audit trail.
    let result = if let Some(ref key) = req.key {
        state
            .producer
            .publish_keyed(&req.topic, key, &payload_bytes)
            .await
    } else {
        state.producer.publish(&req.topic, &payload_bytes).await
    };

    result.map_err(|e| VilError::internal(format!("Produce failed: {}", e)))?;

    Ok(VilResponse::ok(ProduceResponse {
        status: "produced".into(),
        topic: req.topic,
        key: req.key.clone(),
        payload_size: payload_bytes.len(),
        total_produced: state.producer.messages_sent(),
        partitioning: if req.key.is_some() {
            "key-based".into()
        } else {
            "round-robin".into()
        },
    }))
}

/// GET /api/kafka/consumer — consumer group configuration and pattern info.
async fn consumer_info(ctx: ServiceCtx) -> VilResponse<ConsumerInfoResponse> {
    let state = ctx.state::<KafkaState>().expect("state type mismatch");
    let mut consumer_api = std::collections::HashMap::new();
    consumer_api.insert(
        "KafkaConsumer::new(config)".into(),
        "Create a consumer for a group/topic".into(),
    );
    consumer_api.insert("consumer.start()".into(), "Start consuming messages".into());
    consumer_api.insert("consumer.stop()".into(), "Graceful stop".into());
    consumer_api.insert(
        "consumer.take_receiver()".into(),
        "Take mpsc receiver for bridge integration".into(),
    );
    consumer_api.insert(
        "consumer.inject_message(msg)".into(),
        "Inject test message (for testing)".into(),
    );

    VilResponse::ok(ConsumerInfoResponse {
        consumer_group: ConsumerGroupInfo {
            group_id: state.consumer_config.group_id.clone(),
            brokers: state.consumer_config.brokers.clone(),
            topic: state.consumer_config.topic.clone(),
            acks: state.consumer_config.acks.clone(),
        },
        stream_processing_pipeline: StreamProcessingPipeline {
            description: "Consumer → Process → Produce pattern".into(),
            steps: vec![
                PipelineStep {
                    step: 1,
                    name: "Consume".into(),
                    description: "KafkaConsumer reads from input topic (events.orders)".into(),
                    code_pattern: "let msg = consumer.take_receiver().unwrap().recv().await;"
                        .into(),
                },
                PipelineStep {
                    step: 2,
                    name: "Process".into(),
                    description: "Apply business logic, transform, enrich, or filter the message"
                        .into(),
                    code_pattern: "let enriched = process(msg.payload).await;".into(),
                },
                PipelineStep {
                    step: 3,
                    name: "Produce".into(),
                    description: "Write processed result to output topic (events.notifications)"
                        .into(),
                    code_pattern:
                        "producer.publish_keyed(\"events.notifications\", &key, &enriched).await;"
                            .into(),
                },
                PipelineStep {
                    step: 4,
                    name: "Bridge (optional)".into(),
                    description: "Forward to Tri-Lane SHM for zero-copy inter-service delivery"
                        .into(),
                    code_pattern: "bridge.bridge(&kafka_msg).await;".into(),
                },
            ],
        },
        consumer_api,
    })
}

/// GET /api/kafka/bridge — Kafka → Tri-Lane bridge status and info.
async fn bridge_status(ctx: ServiceCtx) -> VilResponse<BridgeStatusResponse> {
    let state = ctx.state::<KafkaState>().expect("state type mismatch");
    let mut api = std::collections::HashMap::new();
    api.insert(
        "KafkaBridge::new(target)".into(),
        "Create bridge targeting a service".into(),
    );
    api.insert(
        "bridge.bridge(&msg)".into(),
        "Forward a KafkaMessage to Tri-Lane SHM".into(),
    );
    api.insert(
        "bridge.bridged_count()".into(),
        "Total messages bridged".into(),
    );

    VilResponse::ok(BridgeStatusResponse {
        bridge: KafkaBridgeInfo {
            target_service: state.bridge.target_service().to_string(),
            messages_bridged: state.bridge.bridged_count(),
            status: "active".into(),
        },
        tri_lane_architecture: TriLaneArchitecture {
            description: "Kafka messages are forwarded to the Tri-Lane SHM mesh for zero-copy inter-service delivery".into(),
            lanes: vec![
                TriLaneLane {
                    name: "Data Lane".into(),
                    description: "Large payload transfer via shared memory (zero-copy)".into(),
                },
                TriLaneLane {
                    name: "Trigger Lane".into(),
                    description: "Lightweight notification that data is ready".into(),
                },
                TriLaneLane {
                    name: "Control Lane".into(),
                    description: "Backpressure and flow control signals".into(),
                },
            ],
            flow: "Kafka Consumer → KafkaBridge → SHM Data Lane → Target Service".into(),
        },
        api,
    })
}

// ---------------------------------------------------------------------------
// Main — VX Process-Oriented (VilApp + ServiceProcess)
// ---------------------------------------------------------------------------

#[tokio::main]
async fn main() {
    // Producer config
    let producer_config = KafkaConfig::new("localhost:9092,localhost:9093,localhost:9094");
    let producer = KafkaProducer::new(producer_config.clone())
        .await
        .expect("Kafka producer creation should succeed (implementation mode)");

    // Consumer config
    let consumer_config = KafkaConfig::new("localhost:9092,localhost:9093,localhost:9094")
        .group("order-processor-group")
        .topic("events.orders");

    // Create bridge
    let bridge = KafkaBridge::new("notification-service");

    let state = KafkaState {
        producer: Arc::new(producer),
        bridge: Arc::new(bridge),
        config: producer_config,
        consumer_config,
    };

    // ── Step 2: Define the Kafka service as a Process ────────────────
    let kafka_service = ServiceProcess::new("kafka")
        .prefix("/api")
        .endpoint(Method::GET, "/kafka/config", get(kafka_config))
        .endpoint(Method::POST, "/kafka/produce", post(kafka_produce))
        .endpoint(Method::GET, "/kafka/consumer", get(consumer_info))
        .endpoint(Method::GET, "/kafka/bridge", get(bridge_status))
        .state(state);

    // ── Step 3: Assemble into VilApp and run ───────────────────────
    VilApp::new("kafka-stream")
        .port(8080)
        .service(ServiceProcess::new("root").endpoint(Method::GET, "/", get(index)))
        .service(kafka_service)
        .run()
        .await;
}
