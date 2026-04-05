// ╔════════════════════════════════════════════════════════════════════════╗
// ║  044 — Social Platform Live Notifications (GraphQL Subscriptions)   ║
// ╠════════════════════════════════════════════════════════════════════════╣
// ║  Domain:   Social Platform — Live Notifications                     ║
// ║  Pattern:  VX_APP                                                    ║
// ║  Features: ServiceProcess, ServiceCtx, ShmSlice, VilResponse,       ║
// ║            VilModel, SseHub, sse_stream                              ║
// ╠════════════════════════════════════════════════════════════════════════╣
// ║  Business: GraphQL-style subscription endpoint for real-time push    ║
// ║  notifications on a social platform. Instead of polling, clients     ║
// ║  subscribe via SSE and receive notifications instantly when:         ║
// ║    - Someone likes their post                                        ║
// ║    - A new follower is added                                         ║
// ║    - A comment is posted on their content                            ║
// ║    - A direct message arrives                                        ║
// ║                                                                      ║
// ║  Endpoints:                                                          ║
// ║    GET  /api/notifications/subscribe → SSE stream of notifications   ║
// ║    POST /api/notifications/publish   → publish notification to all   ║
// ║    GET  /api/notifications/stats     → subscriber count & metrics    ║
// ║                                                                      ║
// ║  Why SSE instead of WebSocket for subscriptions?                     ║
// ║    - Simpler: no upgrade handshake, works through CDNs/proxies       ║
// ║    - Auto-reconnect: browser EventSource reconnects natively         ║
// ║    - Sufficient: notifications are server-to-client push only        ║
// ║    - SseHub handles fan-out — one broadcast reaches all subscribers  ║
// ╚════════════════════════════════════════════════════════════════════════╝
//
// Run:   cargo run -p vil-basic-graphql-subscriptions
// Test:
//   # Terminal 1: subscribe to live notification feed
//   curl -N http://localhost:8080/api/notifications/subscribe
//   # Terminal 2: publish a notification
//   curl -X POST http://localhost:8080/api/notifications/publish \
//     -H 'Content-Type: application/json' \
//     -d '{"kind":"like","from_user":"alice","to_user":"bob","content":"liked your photo"}'
//   # Terminal 3: check stats
//   curl http://localhost:8080/api/notifications/stats

use std::convert::Infallible;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use vil_server::axum;
use vil_server::prelude::*;

// ── Business Domain Faults ──────────────────────────────────────────────

#[vil_fault]
pub enum NotificationFault {
    /// Failed to broadcast notification to subscribers
    PublishFailed,
    /// Invalid notification payload
    InvalidPayload,
}

// ── Models (VilModel = SIMD-ready serialization) ─────────────────────────

#[derive(Clone, Debug, Serialize, Deserialize, VilModel)]
struct Notification {
    kind: String,
    from_user: String,
    to_user: String,
    content: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, VilModel)]
struct PublishResponse {
    published: bool,
    notification_id: u64,
    subscribers_reached: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize, VilModel)]
struct SubscriptionStats {
    active_subscribers: u64,
    total_notifications: u64,
    uptime_secs: u64,
}

// ── Shared State (via ServiceCtx, not Extension<T>) ──────────────────────

struct NotificationState {
    hub: Arc<SseHub>,
    notification_count: AtomicU64,
    started_at: std::time::Instant,
}

// ── Handlers ─────────────────────────────────────────────────────────────

/// GET /notifications/subscribe — Subscribe to live notification stream (SSE).
///
/// Clients open this endpoint and receive real-time notifications as they
/// are published. The connection stays open until the client disconnects.
/// Browser EventSource API handles auto-reconnection natively.
async fn subscribe(ctx: ServiceCtx) -> impl IntoResponse {
    let state = ctx.state::<Arc<NotificationState>>().expect("NotificationState");
    let hub = state.hub.clone();
    let mut rx = hub.subscribe();

    let stream = async_stream::stream! {
        while let Ok(event) = rx.recv().await {
            yield Ok::<_, Infallible>(
                axum::response::sse::Event::default()
                    .event(&event.topic)
                    .data(event.data)
            );
        }
        // Client disconnected — decrement subscriber count
        hub.client_disconnected();
    };

    sse_stream(stream)
}

/// POST /notifications/publish — Publish a notification to all subscribers.
///
/// ShmSlice: zero-copy body from ExchangeHeap (not Json<T>).
/// ServiceCtx: state access via ctx.state (not Extension<T>).
async fn publish(
    ctx: ServiceCtx,
    body: ShmSlice,
) -> HandlerResult<VilResponse<PublishResponse>> {
    let notification: Notification = body.json()
        .map_err(|_| VilError::bad_request(
            "invalid JSON — expected {kind, from_user, to_user, content}"
        ))?;

    if notification.kind.is_empty() || notification.from_user.is_empty() {
        return Err(VilError::bad_request("kind and from_user must not be empty"));
    }

    let state = ctx.state::<Arc<NotificationState>>()
        .map_err(|_| VilError::internal("state not found"))?;

    let notification_id = state.notification_count.fetch_add(1, Ordering::Relaxed) + 1;

    // Broadcast to ALL connected subscribers on the "notification" topic.
    // SseHub handles fan-out — one write reaches all subscribers.
    let json = serde_json::to_string(&notification).unwrap_or_default();
    state.hub.broadcast("notification", json);

    Ok(VilResponse::ok(PublishResponse {
        published: true,
        notification_id,
        subscribers_reached: state.hub.connected_clients(),
    }))
}

/// GET /notifications/stats — Subscription metrics.
///
/// Operations teams use this to monitor platform health:
/// how many users are listening, how many notifications have been pushed.
async fn stats(ctx: ServiceCtx) -> HandlerResult<VilResponse<SubscriptionStats>> {
    let state = ctx.state::<Arc<NotificationState>>()
        .map_err(|_| VilError::internal("state not found"))?;

    Ok(VilResponse::ok(SubscriptionStats {
        active_subscribers: state.hub.connected_clients(),
        total_notifications: state.notification_count.load(Ordering::Relaxed),
        uptime_secs: state.started_at.elapsed().as_secs(),
    }))
}

// ── Main ─────────────────────────────────────────────────────────────────

#[tokio::main]
async fn main() {
    println!("╔════════════════════════════════════════════════════════════════════════╗");
    println!("║  044 — Social Platform Live Notifications (GraphQL Subscriptions)    ║");
    println!("╠════════════════════════════════════════════════════════════════════════╣");
    println!("║  GET  /api/notifications/subscribe → SSE stream (real-time push)     ║");
    println!("║  POST /api/notifications/publish   → broadcast to all subscribers    ║");
    println!("║  GET  /api/notifications/stats     → subscriber count & metrics      ║");
    println!("╚════════════════════════════════════════════════════════════════════════╝");

    let hub = Arc::new(SseHub::new(1024));
    let state = Arc::new(NotificationState {
        hub,
        notification_count: AtomicU64::new(0),
        started_at: std::time::Instant::now(),
    });

    let svc = ServiceProcess::new("notifications")
        .endpoint(Method::GET, "/notifications/subscribe", get(subscribe))
        .endpoint(Method::POST, "/notifications/publish", post(publish))
        .endpoint(Method::GET, "/notifications/stats", get(stats))
        .state(state);

    VilApp::new("social-notifications")
        .port(8080)
        .service(svc)
        .run()
        .await;
}
