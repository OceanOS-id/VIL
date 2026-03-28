// ╔════════════════════════════════════════════════════════════╗
// ║  028 — Live Auction Platform (SSE Streaming)              ║
// ╠════════════════════════════════════════════════════════════╣
// ║  Domain:   E-Commerce / Real-Time Auctions                ║
// ║  Pattern:  VX_APP                                         ║
// ║  Token:    N/A (HTTP server)                              ║
// ║  Macros:   ShmSlice, ServiceCtx, SseEvent, SseHub         ║
// ║  Unique:   Server-side SSE broadcast via SseHub            ║
// ╚════════════════════════════════════════════════════════════╝
//
// Business Context:
//   Real-time bid updates pushed to all connected bidders via SSE
//   (Server-Sent Events). In a live auction platform:
//
//   - Bidders connect via GET /stream and receive real-time bid updates
//   - When a new bid is placed via POST /publish, ALL connected
//     bidders instantly see the updated price
//   - The stats endpoint shows current auction participation metrics
//
//   This pattern is used in:
//   - Live auctions (eBay-style real-time bidding)
//   - Stock trading dashboards (real-time price feeds)
//   - Sports betting (live odds updates)
//   - Collaborative editing (Google Docs-style cursors)
//
// Why SseHub instead of WebSocket?
//   SSE is simpler than WebSocket for server-to-client push:
//   - Works through corporate proxies and CDNs
//   - Auto-reconnects on connection drop (browser-native)
//   - No upgrade handshake overhead
//   - Sufficient when clients only need to RECEIVE updates
//   SseHub manages the broadcast fan-out efficiently — one write
//   reaches all connected bidders without per-client loops.
//
// Test:
//   # Terminal 1: bidder subscribes to live auction feed
//   curl -N http://localhost:8080/api/events/stream
//   # Terminal 2: new bid placed on auction item
//   curl -X POST http://localhost:8080/api/events/publish \
//     -H 'Content-Type: application/json' -d '{"message":"hello"}'

use std::convert::Infallible;
use std::sync::Arc;
use vil_server::axum;
use vil_server::prelude::*;

// Auction fault types — typed faults enable automated alerting
// when bid publishing fails or bidder streams disconnect unexpectedly.
#[vil_fault]
pub enum SseFault {
    PublishFailed, // Failed to broadcast bid update to subscribers
    StreamClosed,  // Bidder's SSE connection dropped unexpectedly
}

// Result of publishing a bid update. Includes the number of
// currently connected bidders who received the update in real-time.
#[derive(Serialize)]
struct PublishResult {
    published: bool,
    clients: u64,
}

// Auction platform statistics: how many bidders are watching,
// and how many bid events have been broadcast in this session.
#[derive(Serialize)]
struct StatsResult {
    connected_clients: u64,
    total_events: u64,
}

/// POST /publish — Place a new bid (broadcast to all connected bidders).
/// When a bidder submits a new price, SseHub instantly pushes the update
/// to every connected client's SSE stream. This ensures all bidders see
/// the same price at the same time — critical for auction fairness.
async fn publish(ctx: ServiceCtx, body: ShmSlice) -> Result<VilResponse<PublishResult>, VilError> {
    // ServiceCtx carries shared state injected during service setup.
    // Here it provides access to the SseHub for broadcast fan-out.
    let hub = ctx.state::<Arc<SseHub>>().expect("SseHub");
    let msg: serde_json::Value = body
        .json()
        .map_err(|_| VilError::bad_request("invalid JSON"))?;
    let text = serde_json::to_string(&msg).unwrap_or_default();

    // Broadcast to ALL connected bidders on the "events" topic.
    // SseHub handles fan-out internally — no per-client iteration needed.
    hub.broadcast("events", text);

    Ok(VilResponse::ok(PublishResult {
        published: true,
        clients: hub.connected_clients(),
    }))
}

/// GET /stream — Subscribe to live auction bid feed (SSE long-lived connection).
/// Each bidder opens this endpoint and receives real-time updates as
/// other bidders place bids. The connection stays open until the bidder
/// disconnects. Browser EventSource API handles auto-reconnection.
async fn stream(ctx: ServiceCtx) -> impl IntoResponse {
    let hub = ctx.state::<Arc<SseHub>>().expect("SseHub");

    // Subscribe to the SseHub's broadcast channel. Each subscriber
    // gets an independent receiver — backpressure is per-client.
    let mut rx = hub.subscribe();

    // Convert SseHub events to axum SSE events for HTTP streaming.
    // The async_stream crate provides zero-allocation streaming.
    let stream = async_stream::stream! {
        while let Ok(event) = rx.recv().await {
            // Convert streaming::SseEvent -> axum SSE Event
            yield Ok::<_, Infallible>(
                axum::response::sse::Event::default()
                    .event(&event.topic)
                    .data(event.data)
            );
        }
    };

    sse_stream(stream)
}

/// GET /stats — Auction platform metrics. Shows how many bidders are
/// currently connected and how many bid events have been broadcast.
/// Ops teams use this to monitor auction health and plan capacity.
async fn stats(ctx: ServiceCtx) -> VilResponse<StatsResult> {
    let hub = ctx.state::<Arc<SseHub>>().expect("SseHub");
    VilResponse::ok(StatsResult {
        connected_clients: hub.connected_clients(),
        total_events: hub.total_events(),
    })
}

#[tokio::main]
async fn main() {
    // Create the SseHub with a buffer of 1024 events. In a live auction,
    // this buffer ensures late-joining bidders can catch up on recent
    // bid history without missing critical price changes.
    let hub = Arc::new(SseHub::new(1024));

    // The "events" ServiceProcess handles all auction streaming.
    // Shared state (SseHub) is injected via .state() and automatically
    // available in all handlers through ServiceCtx extraction.
    let svc = ServiceProcess::new("events")
        .state(hub)
        .endpoint(Method::POST, "/publish", post(publish))
        .endpoint(Method::GET, "/stream", get(stream))
        .endpoint(Method::GET, "/stats", get(stats));

    // Port 8080: the auction platform's internal service port.
    // In production, a reverse proxy (nginx/Caddy) would handle TLS
    // termination and connection limits for the SSE streams.
    VilApp::new("sse-hub-demo")
        .port(8080)
        .service(svc)
        .run()
        .await;
}
