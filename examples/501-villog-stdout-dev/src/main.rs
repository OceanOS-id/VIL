// =============================================================================
// example-501-villog-stdout-dev — VIL Log stdout drain in dev (pretty) mode
// =============================================================================
//
// Demonstrates semantic log emission using:
//   - app_log!  — structured business events
//   - access_log! — HTTP request/response logs
//   - ai_log!   — LLM operation logs
//
// Output goes to stdout in pretty (colored, multi-line) format.
// =============================================================================

use vil_log::drain::{StdoutDrain, StdoutFormat};
use vil_log::runtime::init_logging;
use vil_log::{
    app_log, access_log, ai_log,
    AccessPayload, AiPayload,
    LogConfig, LogLevel,
};

#[tokio::main]
async fn main() {
    let config = LogConfig {
        ring_slots:        4096,
        level:             LogLevel::Debug,
        batch_size:        100,
        flush_interval_ms: 100,
        threads: None,
    };

    let drain = StdoutDrain::new(StdoutFormat::Pretty);
    let _task = init_logging(config, drain);

    // --- Business events via app_log! ---
    app_log!(Info,  "order.created",  { order_id: 12345u64, amount: 50000u64, currency: "IDR" });
    app_log!(Warn,  "payment.retry",  { order_id: 12345u64, attempt: 3u32 });
    app_log!(Error, "inventory.insufficient", { sku: "SKU-001", requested: 10u32, available: 3u32 });
    app_log!(Debug, "cart.item_added", { user_id: 99u64, product_id: 7u32, qty: 2u32 });

    // --- HTTP access log ---
    access_log!(Info, AccessPayload {
        method:         1, // POST
        status_code:    201,
        protocol:       0, // HTTP/1.1
        duration_us:    2_300,
        request_bytes:  256,
        response_bytes: 128,
        path_hash:      register_str("/api/orders"),
        route_hash:     register_str("/api/orders"),
        authenticated:  1,
        ..AccessPayload::default()
    });

    access_log!(Warn, AccessPayload {
        method:         0, // GET
        status_code:    404,
        protocol:       0,
        duration_us:    450,
        path_hash:      register_str("/api/products/9999"),
        route_hash:     register_str("/api/products/:id"),
        ..AccessPayload::default()
    });

    // --- AI/LLM inference log ---
    ai_log!(Info, AiPayload {
        provider_hash:   register_str("openai"),
        model_hash:      register_str("gpt-4o"),
        input_tokens:    150,
        output_tokens:   500,
        latency_us:      1_200_000, // 1.2 seconds
        cost_micro_usd:  350,
        op_type:         0, // chat
        provider_status: 200,
        ..AiPayload::default()
    });

    // Give the drain task time to flush before exit
    tokio::time::sleep(std::time::Duration::from_millis(500)).await;
    println!("\n-- VIL Log stdout dev demo complete --");
}
