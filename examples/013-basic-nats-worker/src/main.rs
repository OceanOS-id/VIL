// ╔════════════════════════════════════════════════════════════╗
// ║  013 — Event-Driven Order Processing (NATS)               ║
// ╠════════════════════════════════════════════════════════════╣
// ║  Pattern:  VX_APP                                           ║
// ║  Token:    N/A (HTTP server)                                ║
// ║  Features: ShmSlice, ServiceCtx, VilResponse                ║
// ║  Domain:   Order events published to NATS subjects,         ║
// ║            workers process asynchronously                   ║
// ╚════════════════════════════════════════════════════════════╝
// basic-usage-nats-worker — NATS Pub/Sub + JetStream Consumer (VX Process-Oriented)
// =============================================================================
//
// BUSINESS CONTEXT:
//   E-commerce order processing pipeline. When a customer places an order,
//   the order service publishes to "events.order.created". Downstream workers
//   (inventory reservation, payment processing, notification sender) subscribe
//   via JetStream durable consumers for at-least-once delivery. The KV store
//   holds feature flags and session tokens for the order flow.
//
// Demonstrates vil_mq_nats integration for NATS Core pub/sub, JetStream
// persistent streaming, and KV store using the VX Process-Oriented architecture
// (VilApp + ServiceProcess). The NATS client uses an in-memory implementation, so
// Requires: NATS server (testsuite: NATS_URL=nats://localhost:19222, or default :4222).
//
// Features demonstrated:
//   - NatsConfig — connection setup with auth options
//   - NatsClient — publish, subscribe, request/reply
//   - JetStreamClient — streams, durable consumers, ack/nack
//   - KvStore — distributed key-value (bucket-based)
//   - NatsBridge — NATS → Tri-Lane SHM zero-copy bridge
//
// Routes:
//   GET  /                    → overview
//   GET  /api/nats/config     → NATS connection status
//   POST /api/nats/publish    → publish a message to a subject
//   GET  /api/nats/jetstream  → JetStream stream configuration info
//   GET  /api/nats/kv         → KV store demo (put/get cycle)
//
// Built-in endpoints (auto-provided by VilServer):
//   GET  /health, /ready, /metrics, /info
//
// Run:
//   cargo run -p basic-usage-nats-worker
//
// Test:
//   curl http://localhost:8080/
//   curl http://localhost:8080/api/nats/config
//   curl -X POST http://localhost:8080/api/nats/publish \
//     -H 'Content-Type: application/json' \
//     -d '{"subject":"events.order.created","payload":{"order_id":42,"total":99.99}}'
//   curl http://localhost:8080/api/nats/jetstream
//   curl http://localhost:8080/api/nats/kv
// =============================================================================

use vil_server::axum::extract::Extension;
use vil_server::prelude::*;

use vil_mq_nats::jetstream::StreamConfig;
use vil_mq_nats::{JetStreamClient, KvStore, NatsBridge, NatsClient, NatsConfig};

use std::sync::Arc;

// ---------------------------------------------------------------------------
// Shared state
// ---------------------------------------------------------------------------

#[derive(Clone)]
struct NatsState {
    client: Arc<NatsClient>,
    jetstream: Arc<JetStreamClient>,
    kv: Arc<KvStore>,
    bridge: Arc<NatsBridge>,
    config: NatsConfig,
}

// ---------------------------------------------------------------------------
// Response types
// ---------------------------------------------------------------------------

#[derive(Clone, Debug, Serialize, Deserialize, VilModel)]
struct NatsConfigResponse {
    connection: NatsConnectionInfo,
    metrics: NatsMetrics,
    bridge: BridgeInfo,
    jetstream: JetStreamSummary,
    kv: KvSummary,
}

#[derive(Clone, Debug, Serialize, Deserialize, VilModel)]
struct NatsConnectionInfo {
    url: String,
    client_name: String,
    connected: bool,
    tls: bool,
    max_reconnects: u32,
    buffer_size: usize,
}

#[derive(Clone, Debug, Serialize, Deserialize, VilModel)]
struct NatsMetrics {
    messages_published: u64,
    messages_received: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize, VilModel)]
struct BridgeInfo {
    target_service: String,
    messages_bridged: u64,
    description: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, VilModel)]
struct JetStreamSummary {
    streams: Vec<String>,
    stream_count: usize,
}

#[derive(Clone, Debug, Serialize, Deserialize, VilModel)]
struct KvSummary {
    bucket: String,
    keys: usize,
}

#[derive(Clone, Debug, Serialize, Deserialize, VilModel)]
struct PublishResponse {
    status: String,
    subject: String,
    payload_size: usize,
    total_published: u64,
    bridged_to_shm: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize, VilModel)]
struct StreamConfigInfo {
    name: String,
    subjects: Vec<String>,
    retention: String,
    max_msgs: i64,
    max_bytes: i64,
    description: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, VilModel)]
struct ConsumerPattern {
    durable_name: String,
    filter_subject: String,
    ack_policy: String,
    deliver_policy: String,
    description: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, VilModel)]
struct JetStreamInfoResponse {
    jetstream: JetStreamDetail,
    api_surface: std::collections::HashMap<String, String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, VilModel)]
struct JetStreamDetail {
    description: String,
    streams: Vec<String>,
    stream_count: usize,
    stream_configs: Vec<StreamConfigInfo>,
    consumer_pattern: ConsumerPattern,
}

#[derive(Clone, Debug, Serialize, Deserialize, VilModel)]
struct KvEntryInfo {
    value: String,
    revision: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize, VilModel)]
struct KvStoreInfo {
    bucket: String,
    total_keys: usize,
    keys: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, VilModel)]
struct KvDemoResponse {
    kv_store: KvStoreInfo,
    demo_entries: std::collections::HashMap<String, Option<KvEntryInfo>>,
    api_surface: std::collections::HashMap<String, String>,
}

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

/// GET / — overview of the NATS worker example.
async fn index() -> &'static str {
    "VIL NATS Worker Example\n\
     ============================\n\n\
     Demonstrates NATS pub/sub, JetStream, and KV store patterns.\n\n\
     Endpoints:\n\
     - GET  /api/nats/config     — connection status and metrics\n\
     - POST /api/nats/publish    — publish a message to a NATS subject\n\
     - GET  /api/nats/jetstream  — JetStream stream configuration\n\
     - GET  /api/nats/kv         — KV store demo (put/get cycle)\n\n\
     Built-in:\n\
     - GET  /health, /ready, /metrics, /info\n"
}

/// GET /api/nats/config — NatsConfig pattern with all connection fields.
async fn nats_config(ctx: ServiceCtx) -> VilResponse<NatsConfigResponse> {
    let state = ctx.state::<NatsState>().expect("state type mismatch");
    VilResponse::ok(NatsConfigResponse {
        connection: NatsConnectionInfo {
            url: state.config.url.clone(),
            client_name: state.config.client_name.clone(),
            connected: state.client.is_connected(),
            tls: state.config.tls,
            max_reconnects: state.config.max_reconnects,
            buffer_size: state.config.buffer_size,
        },
        metrics: NatsMetrics {
            messages_published: state.client.published_count(),
            messages_received: state.client.received_count(),
        },
        bridge: BridgeInfo {
            target_service: state.bridge.target().to_string(),
            messages_bridged: state.bridge.bridged_count(),
            description: "NatsBridge forwards NATS messages to Tri-Lane SHM mesh".into(),
        },
        jetstream: JetStreamSummary {
            streams: state.jetstream.streams(),
            stream_count: state.jetstream.stream_count(),
        },
        kv: KvSummary {
            bucket: state.kv.bucket().to_string(),
            keys: state.kv.len().await,
        },
    })
}

/// Request body for publishing a message.
#[derive(Clone, Debug, Serialize, Deserialize, VilModel)]
struct PublishRequest {
    subject: String,
    payload: serde_json::Value,
}

/// POST /api/nats/publish — publish a message to a NATS subject.
async fn nats_publish(
    ctx: ServiceCtx,
    body: ShmSlice,
) -> HandlerResult<VilResponse<PublishResponse>> {
    let state = ctx.state::<NatsState>().expect("state type mismatch");
    let req: PublishRequest = body.json().expect("invalid JSON body");
    let payload_bytes = serde_json::to_vec(&req.payload)
        .map_err(|e| VilError::bad_request(format!("Invalid payload: {}", e)))?;

    // Publish via NATS client (in-memory implementation)
    state
        .client
        .publish(&req.subject, &payload_bytes)
        .await
        .map_err(|e| VilError::internal(format!("Publish failed: {}", e)))?;

    // Also bridge to Tri-Lane SHM
    state.bridge.bridge(&req.subject, &payload_bytes).await;

    Ok(VilResponse::ok(PublishResponse {
        status: "published".into(),
        subject: req.subject,
        payload_size: payload_bytes.len(),
        total_published: state.client.published_count(),
        bridged_to_shm: true,
    }))
}

/// GET /api/nats/jetstream — JetStream stream configuration and info.
async fn jetstream_info(ctx: ServiceCtx) -> VilResponse<JetStreamInfoResponse> {
    let state = ctx.state::<NatsState>().expect("state type mismatch");
    let mut api_surface = std::collections::HashMap::new();
    api_surface.insert(
        "JetStreamClient::create_stream".into(),
        "Create a persistent stream".into(),
    );
    api_surface.insert(
        "JetStreamClient::create_consumer".into(),
        "Create a durable consumer on a stream".into(),
    );
    api_surface.insert(
        "JetStreamClient::publish".into(),
        "Publish to a JetStream subject (returns sequence number)".into(),
    );
    api_surface.insert(
        "JsMessage::ack()".into(),
        "Acknowledge message (removes from redelivery queue)".into(),
    );
    api_surface.insert(
        "JsMessage::nack()".into(),
        "Negative-ack (request redelivery)".into(),
    );

    VilResponse::ok(JetStreamInfoResponse {
        jetstream: JetStreamDetail {
            description: "JetStream provides persistent streaming with at-least-once delivery"
                .into(),
            streams: state.jetstream.streams(),
            stream_count: state.jetstream.stream_count(),
            stream_configs: vec![
                StreamConfigInfo {
                    name: "ORDERS".into(),
                    subjects: vec!["events.order.>".into()],
                    retention: "limits".into(),
                    max_msgs: -1,
                    max_bytes: -1,
                    description: "All order lifecycle events".into(),
                },
                StreamConfigInfo {
                    name: "NOTIFICATIONS".into(),
                    subjects: vec!["notifications.>".into()],
                    retention: "limits".into(),
                    max_msgs: 100000,
                    max_bytes: 104857600,
                    description: "User notification events (100k msg limit)".into(),
                },
            ],
            consumer_pattern: ConsumerPattern {
                durable_name: "order-processor".into(),
                filter_subject: "events.order.created".into(),
                ack_policy: "explicit".into(),
                deliver_policy: "all".into(),
                description: "Durable consumer processes order creation events".into(),
            },
        },
        api_surface,
    })
}

/// GET /api/nats/kv — KV store demo with a put/get cycle.
async fn kv_demo(ctx: ServiceCtx) -> VilResponse<KvDemoResponse> {
    let state = ctx.state::<NatsState>().expect("state type mismatch");
    // Demonstrate put/get cycle — real-world uses:
    //   feature_flags: toggle checkout flow A/B test
    //   session: track user auth tokens (fast lookup vs DB round-trip)
    //   rate_limit: API key throttling counters
    let _ = state
        .kv
        .put(
            "config:feature_flags",
            b"{\"dark_mode\":true,\"beta\":false}",
        )
        .await;
    let _ = state
        .kv
        .put(
            "session:user-101",
            b"{\"token\":\"abc123\",\"expires\":3600}",
        )
        .await;
    let _ = state
        .kv
        .put(
            "rate_limit:api-key-xyz",
            b"{\"remaining\":98,\"reset\":1700000000}",
        )
        .await;

    // Read back
    let feature_flags = state.kv.get("config:feature_flags").await;
    let session = state.kv.get("session:user-101").await;

    let mut demo_entries = std::collections::HashMap::new();
    demo_entries.insert(
        "config:feature_flags".to_string(),
        feature_flags.as_ref().map(|e| KvEntryInfo {
            value: String::from_utf8_lossy(&e.value).to_string(),
            revision: e.revision,
        }),
    );
    demo_entries.insert(
        "session:user-101".to_string(),
        session.as_ref().map(|e| KvEntryInfo {
            value: String::from_utf8_lossy(&e.value).to_string(),
            revision: e.revision,
        }),
    );

    let mut api_surface = std::collections::HashMap::new();
    api_surface.insert(
        "KvStore::put(key, value)".into(),
        "Store a key-value pair (returns revision)".into(),
    );
    api_surface.insert(
        "KvStore::get(key)".into(),
        "Get value by key (returns KvEntry with revision)".into(),
    );
    api_surface.insert("KvStore::delete(key)".into(), "Delete a key".into());
    api_surface.insert(
        "KvStore::keys()".into(),
        "List all keys in the bucket".into(),
    );
    api_surface.insert(
        "KvStore::watch()".into(),
        "Watch for changes (broadcast receiver)".into(),
    );

    VilResponse::ok(KvDemoResponse {
        kv_store: KvStoreInfo {
            bucket: state.kv.bucket().to_string(),
            total_keys: state.kv.len().await,
            keys: state.kv.keys().await,
        },
        demo_entries,
        api_surface,
    })
}

// ---------------------------------------------------------------------------
// Main — VX Process-Oriented (VilApp + ServiceProcess)
// ---------------------------------------------------------------------------

#[tokio::main]
async fn main() {
    // Configure NATS connection (testsuite: port 19222, default: 4222)
    let nats_url = std::env::var("NATS_URL")
        .unwrap_or_else(|_| "nats://localhost:4222".into());
    let nats_cfg = NatsConfig::new(&nats_url).name("vil-nats-worker");

    // Connect to NATS server. Graceful degradation if unreachable.
    let client = NatsClient::connect(nats_cfg.clone())
        .await
        .expect("NATS connection failed — start NATS: cd vil-testsuite/infra && ./up.sh");

    // Create JetStream client and register streams.
    // ORDERS stream: captures all order lifecycle events (created, paid, shipped, delivered).
    // NOTIFICATIONS stream: bounded to 100k msgs to prevent unbounded growth.
    let jetstream = JetStreamClient::new(client.inner());
    let _ = jetstream
        .create_stream(StreamConfig {
            name: "ORDERS".into(),
            subjects: vec!["events.order.>".into()],
            retention: "limits".into(),
            max_msgs: -1,
            max_bytes: -1,
        })
        .await;
    let _ = jetstream
        .create_stream(StreamConfig {
            name: "NOTIFICATIONS".into(),
            subjects: vec!["notifications.>".into()],
            retention: "limits".into(),
            max_msgs: 100000,
            max_bytes: 104857600,
        })
        .await;

    // Create KV store
    let kv = KvStore::new(jetstream.inner(), "vil-config")
        .await
        .expect("KV store creation should succeed");

    // Create bridge to Tri-Lane SHM
    let bridge = NatsBridge::new("order-service");

    let state = NatsState {
        client: Arc::new(client),
        jetstream: Arc::new(jetstream),
        kv: Arc::new(kv),
        bridge: Arc::new(bridge),
        config: nats_cfg,
    };

    // ── Step 2: Define the NATS service as a Process ─────────────────
    let nats_service = ServiceProcess::new("nats")
        .prefix("/api")
        .endpoint(Method::GET, "/nats/config", get(nats_config))
        .endpoint(Method::POST, "/nats/publish", post(nats_publish))
        .endpoint(Method::GET, "/nats/jetstream", get(jetstream_info))
        .endpoint(Method::GET, "/nats/kv", get(kv_demo))
        .state(state);

    // ── Step 3: Assemble into VilApp and run ───────────────────────
    VilApp::new("nats-worker")
        .port(8080)
        .service(ServiceProcess::new("root").endpoint(Method::GET, "/", get(index)))
        .service(nats_service)
        .run()
        .await;
}
