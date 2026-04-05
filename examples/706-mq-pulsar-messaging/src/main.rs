// ╔════════════════════════════════════════════════════════════╗
// ║  706 — FinTech: Transaction Event Bus (Pulsar Pattern)     ║
// ╠════════════════════════════════════════════════════════════╣
// ║  Domain:   FinTech — Transaction Event Bus                  ║
// ║  Pattern:  VX_APP                                           ║
// ║  Features: ServiceCtx, ShmSlice, VilResponse, in-memory    ║
// ║            channel as Pulsar-pattern message queue           ║
// ╠════════════════════════════════════════════════════════════╣
// ║  Business: Publish transaction events (payment, refund,     ║
// ║  transfer) to a topic, consume with consumer groups.        ║
// ║  In production, swap for vil_mq_pulsar::{PulsarProducer,   ║
// ║  PulsarConsumer} backed by Apache Pulsar. This demo uses    ║
// ║  in-memory tokio channels so it runs without a broker.      ║
// ╚════════════════════════════════════════════════════════════╝
//
// Production pattern with real Apache Pulsar:
//
//   use vil_mq_pulsar::{PulsarClient, PulsarConfig, PulsarProducer, PulsarConsumer};
//   let config = PulsarConfig { url: "pulsar://localhost:6650".into(), .. };
//   let client = PulsarClient::new(config).await.expect("connect");
//   let producer = client.producer("persistent://fintech/txn/events").await;
//   let consumer = client.consumer("persistent://fintech/txn/events", "audit-group").await;
//
// Run:   cargo run -p vil-mq-pulsar-messaging
// Test:
//   curl -X POST http://localhost:8080/api/events/publish \
//     -H 'Content-Type: application/json' \
//     -d '{"event_type":"payment","transaction_id":"TXN-001","amount":250.00,"currency":"USD","from_account":"ACC-1234","to_account":"ACC-5678"}'
//
//   curl http://localhost:8080/api/events/consume
//
//   curl http://localhost:8080/api/events/stats

use std::collections::VecDeque;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use tokio::sync::RwLock;
use vil_server::prelude::*;

// ── Event Models ─────────────────────────────────────────────────────────

#[derive(Clone, Debug, Serialize, Deserialize, VilModel)]
struct TransactionEvent {
    event_id: String,
    event_type: String,
    transaction_id: String,
    amount: f64,
    currency: String,
    from_account: String,
    to_account: String,
    published_at: String,
    status: String,
}

#[derive(Debug, Deserialize)]
struct PublishRequest {
    event_type: String,
    transaction_id: String,
    amount: f64,
    #[serde(default = "default_currency")]
    currency: String,
    from_account: String,
    to_account: String,
}

fn default_currency() -> String {
    "USD".into()
}

#[derive(Clone, Debug, Serialize, Deserialize, VilModel)]
struct PublishResponse {
    status: String,
    event_id: String,
    topic: String,
    queue_depth: usize,
}

#[derive(Clone, Debug, Serialize, Deserialize, VilModel)]
struct ConsumeResponse {
    status: String,
    event: Option<TransactionEvent>,
    remaining: usize,
    consumer_group: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, VilModel)]
struct StatsResponse {
    total_published: u64,
    total_consumed: u64,
    queue_depth: usize,
    topics: Vec<TopicStats>,
    note: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, VilModel)]
struct TopicStats {
    topic: String,
    published: u64,
    description: String,
}

// ── In-Memory Message Queue ──────────────────────────────────────────────
// Mimics Pulsar's persistent topic with consumer group consumption.
// In production, replace with PulsarProducer/PulsarConsumer from vil_mq_pulsar.

struct EventBus {
    /// Pending events (consumed in FIFO order, like a Pulsar subscription).
    queue: RwLock<VecDeque<TransactionEvent>>,
    /// Counter for total published events.
    total_published: AtomicU64,
    /// Counter for total consumed events.
    total_consumed: AtomicU64,
    /// Counters per event type.
    payment_count: AtomicU64,
    refund_count: AtomicU64,
    transfer_count: AtomicU64,
}

impl EventBus {
    fn new() -> Self {
        Self {
            queue: RwLock::new(VecDeque::new()),
            total_published: AtomicU64::new(0),
            total_consumed: AtomicU64::new(0),
            payment_count: AtomicU64::new(0),
            refund_count: AtomicU64::new(0),
            transfer_count: AtomicU64::new(0),
        }
    }

    async fn publish(&self, event: TransactionEvent) -> usize {
        match event.event_type.as_str() {
            "payment" => { self.payment_count.fetch_add(1, Ordering::Relaxed); }
            "refund" => { self.refund_count.fetch_add(1, Ordering::Relaxed); }
            "transfer" => { self.transfer_count.fetch_add(1, Ordering::Relaxed); }
            _ => {}
        }
        self.total_published.fetch_add(1, Ordering::Relaxed);
        let mut q = self.queue.write().await;
        q.push_back(event);
        q.len()
    }

    async fn consume(&self) -> (Option<TransactionEvent>, usize) {
        let mut q = self.queue.write().await;
        let event = q.pop_front();
        if event.is_some() {
            self.total_consumed.fetch_add(1, Ordering::Relaxed);
        }
        let remaining = q.len();
        (event, remaining)
    }

    async fn queue_depth(&self) -> usize {
        self.queue.read().await.len()
    }
}

struct AppState {
    bus: EventBus,
    event_counter: AtomicU64,
}

// ── Handlers ─────────────────────────────────────────────────────────────

/// POST /api/events/publish — publish a transaction event.
async fn publish_event(
    ctx: ServiceCtx,
    body: ShmSlice,
) -> HandlerResult<VilResponse<PublishResponse>> {
    let req: PublishRequest = body.json()
        .map_err(|_| VilError::bad_request(
            "invalid JSON — expected {\"event_type\", \"transaction_id\", \"amount\", \"from_account\", \"to_account\"}"
        ))?;

    // Validate event type
    if !["payment", "refund", "transfer"].contains(&req.event_type.as_str()) {
        return Err(VilError::bad_request(
            "event_type must be one of: payment, refund, transfer"
        ));
    }

    let state = ctx.state::<Arc<AppState>>()
        .map_err(|_| VilError::internal("state not found"))?;

    let seq = state.event_counter.fetch_add(1, Ordering::Relaxed);
    let event_id = format!("EVT-{:06}", seq + 1);

    let event = TransactionEvent {
        event_id: event_id.clone(),
        event_type: req.event_type.clone(),
        transaction_id: req.transaction_id,
        amount: req.amount,
        currency: req.currency,
        from_account: req.from_account,
        to_account: req.to_account,
        published_at: "2026-04-05T10:00:00Z".into(),
        status: "published".into(),
    };

    let topic = format!("persistent://fintech/txn/{}", req.event_type);
    let queue_depth = state.bus.publish(event).await;

    Ok(VilResponse::ok(PublishResponse {
        status: "published".into(),
        event_id,
        topic,
        queue_depth,
    }))
}

/// GET /api/events/consume — consume next event from queue (like Pulsar shared subscription).
async fn consume_event(
    ctx: ServiceCtx,
) -> HandlerResult<VilResponse<ConsumeResponse>> {
    let state = ctx.state::<Arc<AppState>>()
        .map_err(|_| VilError::internal("state not found"))?;

    let (event, remaining) = state.bus.consume().await;

    let status = if event.is_some() { "consumed" } else { "empty" };

    Ok(VilResponse::ok(ConsumeResponse {
        status: status.into(),
        event,
        remaining,
        consumer_group: "audit-consumer-group".into(),
    }))
}

/// GET /api/events/stats — event bus statistics.
async fn event_stats(
    ctx: ServiceCtx,
) -> HandlerResult<VilResponse<StatsResponse>> {
    let state = ctx.state::<Arc<AppState>>()
        .map_err(|_| VilError::internal("state not found"))?;

    let depth = state.bus.queue_depth().await;

    Ok(VilResponse::ok(StatsResponse {
        total_published: state.bus.total_published.load(Ordering::Relaxed),
        total_consumed: state.bus.total_consumed.load(Ordering::Relaxed),
        queue_depth: depth,
        topics: vec![
            TopicStats {
                topic: "persistent://fintech/txn/payment".into(),
                published: state.bus.payment_count.load(Ordering::Relaxed),
                description: "Payment transaction events (card charges, ACH, wire)".into(),
            },
            TopicStats {
                topic: "persistent://fintech/txn/refund".into(),
                published: state.bus.refund_count.load(Ordering::Relaxed),
                description: "Refund events (full/partial refunds, chargebacks)".into(),
            },
            TopicStats {
                topic: "persistent://fintech/txn/transfer".into(),
                published: state.bus.transfer_count.load(Ordering::Relaxed),
                description: "Internal transfer events (account-to-account)".into(),
            },
        ],
        note: "Demo mode: in-memory channel. Production: use vil_mq_pulsar with PulsarProducer/PulsarConsumer.".into(),
    }))
}

// ── Main ─────────────────────────────────────────────────────────────────

#[tokio::main]
async fn main() {
    let state = Arc::new(AppState {
        bus: EventBus::new(),
        event_counter: AtomicU64::new(0),
    });

    let svc = ServiceProcess::new("events")
        .prefix("/api")
        .endpoint(Method::POST, "/events/publish", post(publish_event))
        .endpoint(Method::GET, "/events/consume", get(consume_event))
        .endpoint(Method::GET, "/events/stats", get(event_stats))
        .state(state);

    VilApp::new("pulsar-transaction-bus")
        .port(8080)
        .observer(true)
        .service(svc)
        .run()
        .await;
}
