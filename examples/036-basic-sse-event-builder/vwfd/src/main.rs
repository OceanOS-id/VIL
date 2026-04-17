// 036 — Stock Market Ticker (NativeCode)
// Business logic matches standard src/main.rs:
//   GET /api/ticker/info → service info with stream_url, event_types, client_example
//   GET /api/ticker/stream → SSE not possible in buffered workflow, return snapshot instead
use serde_json::json;

#[tokio::main]
async fn main() {
    vil_vwfd::app("examples/036-basic-sse-event-builder/vwfd/workflows", 8080)
        .native("ticker_info", |_| {
            Ok(json!({
                "service": "VIL Stock Market Ticker — Real-Time SSE Demo",
                "stream_url": "/api/ticker/stream",
                "event_types": [
                    "data (raw price ticks)",
                    "named:trade (trade notifications)",
                    "json (full StockQuote)",
                    "named_json:alert (price alerts)",
                    "named_json:quote (streaming quotes)"
                ],
                "symbols": ["AAPL", "GOOGL", "MSFT"],
                "client_example": "curl -N http://localhost:8080/api/ticker/stream"
            }))
        })
        .native("ticker_stream_snapshot", |_| {
            // SSE streaming not available in buffered workflow mode.
            // Return a snapshot of current prices instead.
            Ok(json!({
                "mode": "snapshot",
                "note": "SSE streaming available in standard mode. This returns a point-in-time snapshot.",
                "prices": [
                    {"symbol": "AAPL", "price": 182.50, "volume": 1250000, "bid": 182.45, "ask": 182.55, "change_percent": 1.2},
                    {"symbol": "GOOGL", "price": 141.20, "volume": 890000, "bid": 141.15, "ask": 141.25, "change_percent": -0.3},
                    {"symbol": "MSFT", "price": 415.80, "volume": 2100000, "bid": 415.70, "ask": 415.90, "change_percent": 0.8}
                ]
            }))
        })
        .run().await;
}
