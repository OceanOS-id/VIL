// 036 — Stock Market Ticker SSE (NativeCode for info, stream via existing workflow)
use serde_json::json;

#[tokio::main]
async fn main() {
    vil_vwfd::app("examples/036-basic-sse-event-builder/vwfd/workflows", 8080)
        .native("ticker_info", |_| {
            Ok(json!({
                "service": "Stock Market Ticker",
                "event_types": ["price_update", "trade", "alert"],
                "symbols": ["BBCA", "TLKM", "BMRI", "ASII"],
            }))
        })
        .native("sse_ticker_stream", |_| {
            Ok(json!({"stream": "started", "format": "sse"}))
        })
        .run().await;
}
