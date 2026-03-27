// ╔════════════════════════════════════════════════════════════════════════╗
// ║  036 — Stock Market Ticker (SSE Event Builder)                      ║
// ╠════════════════════════════════════════════════════════════════════════╣
// ║  Pattern:  VX_APP                                                    ║
// ║  Token:    N/A                                                       ║
// ║  Features: SseEvent::data, ::named, ::json, ::named_json             ║
// ╠════════════════════════════════════════════════════════════════════════╣
// ║  Business: A financial data platform streams live stock prices to    ║
// ║  trading dashboards via Server-Sent Events (SSE). Different event   ║
// ║  types carry different data:                                         ║
// ║                                                                      ║
// ║  Event Types (demonstrating all four SseEvent variants):             ║
// ║    1. SseEvent::data("AAPL:182.50")                                  ║
// ║       → Raw price tick (lightweight, no JSON overhead)               ║
// ║    2. SseEvent::named("trade", "AAPL buy 100@182.50")               ║
// ║       → Trade execution notification (named event for filtering)    ║
// ║    3. SseEvent::json(&quote)                                         ║
// ║       → Full quote object with volume, bid/ask, market cap          ║
// ║    4. SseEvent::named_json("alert", &alert)                         ║
// ║       → Price alert when a stock crosses a threshold                ║
// ║                                                                      ║
// ║  Why SSE for stock tickers:                                          ║
// ║    - Unidirectional: server → client (no client messages needed)    ║
// ║    - Auto-reconnect built into browsers (EventSource API)           ║
// ║    - Simpler than WebSocket for read-only data feeds                ║
// ║    - VIL's sse_stream() adds automatic keep-alive pings             ║
// ╚════════════════════════════════════════════════════════════════════════╝
//
// Run:  cargo run -p vil-basic-sse-event-builder
// Test: curl -N http://localhost:8080/api/ticker/stream

use vil_server::prelude::*;
use std::convert::Infallible;
use std::time::Duration;

// ── Business Domain Types ───────────────────────────────────────────────

/// Full stock quote with market data.
/// Sent as JSON via SseEvent::json() for dashboard rendering.
#[derive(Serialize)]
struct StockQuote {
    symbol: &'static str,
    price: f64,
    volume: u64,
    bid: f64,
    ask: f64,
    change_percent: f64,
}

/// Price alert triggered when a stock crosses a configured threshold.
/// Sent as named JSON via SseEvent::named_json("alert", ...).
#[derive(Serialize)]
struct TradeAlert {
    symbol: &'static str,
    threshold: f64,
    current_price: f64,
    direction: &'static str,
    message: String,
}

// ── Stock Ticker Stream ─────────────────────────────────────────────────

/// SSE stream that simulates a live stock market ticker.
///
/// Demonstrates all FOUR SseEvent builder variants in a real-world context:
/// - data(): raw price ticks (minimal overhead for high-frequency data)
/// - named(): trade executions (clients filter by event name)
/// - json(): full quote objects (structured data for dashboards)
/// - named_json(): price alerts (named + structured for alert systems)
async fn ticker_stream() -> impl IntoResponse {
    let stream = async_stream::stream! {
        // ── VARIANT 1: SseEvent::data() ─────────────────────────────
        // Raw price tick — no event name, no JSON.
        // Used for high-frequency data where every byte counts.
        // Trading algorithms consume these at microsecond speeds.
        yield Ok::<_, Infallible>(SseEvent::data("AAPL:182.50|GOOGL:141.20|MSFT:415.80").unwrap());
        tokio::time::sleep(Duration::from_millis(500)).await;

        // ── VARIANT 2: SseEvent::named() ────────────────────────────
        // Trade execution notification with an event type.
        // Dashboard JavaScript uses EventSource.addEventListener("trade", ...)
        // to filter and display only trade events in the execution blotter.
        yield Ok::<_, Infallible>(SseEvent::named("trade", "AAPL buy 100@182.50 via NASDAQ").unwrap());
        tokio::time::sleep(Duration::from_millis(500)).await;

        yield Ok::<_, Infallible>(SseEvent::named("trade", "GOOGL sell 50@141.20 via NYSE").unwrap());
        tokio::time::sleep(Duration::from_millis(500)).await;

        // ── VARIANT 3: SseEvent::json() ─────────────────────────────
        // Full stock quote as JSON — no event name.
        // Dashboard parses this to update price charts, volume bars,
        // and bid/ask spread indicators.
        yield Ok::<_, Infallible>(SseEvent::json(&StockQuote {
            symbol: "AAPL",
            price: 182.75,
            volume: 1_250_000,
            bid: 182.70,
            ask: 182.80,
            change_percent: 1.34,
        }).unwrap());
        tokio::time::sleep(Duration::from_millis(500)).await;

        yield Ok::<_, Infallible>(SseEvent::json(&StockQuote {
            symbol: "GOOGL",
            price: 141.55,
            volume: 890_000,
            bid: 141.50,
            ask: 141.60,
            change_percent: -0.42,
        }).unwrap());
        tokio::time::sleep(Duration::from_millis(500)).await;

        // ── VARIANT 4: SseEvent::named_json() ──────────────────────
        // Price alert — named event + JSON payload.
        // Alert systems subscribe to EventSource.addEventListener("alert", ...)
        // and trigger notifications (push, SMS, email) when thresholds are crossed.
        yield Ok::<_, Infallible>(SseEvent::named_json("alert", &TradeAlert {
            symbol: "AAPL",
            threshold: 182.00,
            current_price: 182.75,
            direction: "above",
            message: "AAPL crossed above $182.00 — consider taking profit".into(),
        }).unwrap());
        tokio::time::sleep(Duration::from_millis(500)).await;

        // Simulate continuous price updates (as in a real market feed)
        let prices = [183.10, 183.25, 182.90, 183.50, 184.00];
        for (i, price) in prices.iter().enumerate() {
            yield Ok::<_, Infallible>(SseEvent::named_json("quote", &StockQuote {
                symbol: "AAPL",
                price: *price,
                volume: 1_250_000 + (i as u64 * 50_000),
                bid: price - 0.05,
                ask: price + 0.05,
                change_percent: (price - 180.33) / 180.33 * 100.0,
            }).unwrap());
            tokio::time::sleep(Duration::from_millis(300)).await;
        }

        // Market close signal
        yield Ok::<_, Infallible>(SseEvent::named("status", "market_closed — stream ending").unwrap());
    };

    // sse_stream wraps the stream with automatic keep-alive pings (15s default).
    // This prevents proxies/load balancers from closing idle connections.
    sse_stream(stream)
}

/// API info endpoint listing all SSE event types used in this ticker.
async fn ticker_info() -> VilResponse<serde_json::Value> {
    VilResponse::ok(serde_json::json!({
        "service": "Stock Market Ticker",
        "stream_url": "/api/ticker/stream",
        "event_types": [
            "SseEvent::data(text) → raw price tick (SYMBOL:PRICE format)",
            "SseEvent::named('trade', text) → trade execution notification",
            "SseEvent::json(&StockQuote) → full quote object",
            "SseEvent::named_json('alert', &TradeAlert) → price threshold alert",
        ],
        "client_example": "const es = new EventSource('/api/ticker/stream'); es.addEventListener('alert', e => console.log(e.data));"
    }))
}

#[tokio::main]
async fn main() {
    println!("╔════════════════════════════════════════════════════════════════════════╗");
    println!("║  036 — Stock Market Ticker (SSE Event Builder)                       ║");
    println!("╠════════════════════════════════════════════════════════════════════════╣");
    println!("║  data()       → raw price ticks (AAPL:182.50)                        ║");
    println!("║  named()      → trade executions (buy/sell notifications)             ║");
    println!("║  json()       → full StockQuote objects                               ║");
    println!("║  named_json() → TradeAlert when price crosses threshold              ║");
    println!("╚════════════════════════════════════════════════════════════════════════╝");
    println!("  curl -N http://localhost:8080/api/ticker/stream");

    let ticker_svc = ServiceProcess::new("ticker")
        .endpoint(Method::GET, "/stream", get(ticker_stream))
        .endpoint(Method::GET, "/info", get(ticker_info));

    VilApp::new("stock-market-ticker")
        .port(8080)
        .service(ticker_svc)
        .run()
        .await;
}
