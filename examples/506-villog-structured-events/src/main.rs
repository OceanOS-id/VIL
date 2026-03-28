// =============================================================================
// example-506-villog-structured-events — All 7 VIL log categories
// =============================================================================
//
// Demonstrates realistic usage of every semantic log category:
//
//   1. access_log!   — HTTP request/response (API gateway)
//   2. app_log!      — Business events (order lifecycle)
//   3. ai_log!       — LLM inference (chat completion)
//   4. db_log!       — Database operations (SQL queries)
//   5. mq_log!       — Message queue (Kafka consume/publish)
//   6. system_log!   — OS resource metrics (CPU, memory)
//   7. security_log! — Auth and authorization events
//
// Output: pretty stdout so every field is visible.
// =============================================================================

use vil_log::drain::{StdoutDrain, StdoutFormat};
use vil_log::runtime::init_logging;
use vil_log::{
    access_log, app_log, ai_log, db_log, mq_log, system_log, security_log,
    AccessPayload, AiPayload, DbPayload, MqPayload, SystemPayload, SecurityPayload,
    LogConfig, LogLevel,
};

#[tokio::main]
async fn main() {
    let config = LogConfig {
        ring_slots:        8192,
        level:             LogLevel::Trace,
        batch_size:        256,
        flush_interval_ms: 50,
        threads: None,
        dict_path: None,
        fallback_path: None,
        drain_failure_threshold: 3,
    };
    let drain = StdoutDrain::new(StdoutFormat::Pretty);
    let _task = init_logging(config, drain);

    println!("=== VIL Log — All 7 structured event categories ===\n");

    // -------------------------------------------------------------------------
    // 1. ACCESS — incoming API request
    // -------------------------------------------------------------------------
    println!("--- 1. access_log! ---");
    access_log!(Info, AccessPayload {
        method:         1,    // POST
        status_code:    201,
        protocol:       1,    // HTTP/2
        duration_us:    1_850,
        request_bytes:  512,
        response_bytes: 256,
        server_port:    8080,
        route_hash:     register_str("/api/v1/orders"),
        path_hash:      register_str("/api/v1/orders"),
        authenticated:  1,
        cache_status:   0,    // miss
        ..AccessPayload::default()
    });

    // -------------------------------------------------------------------------
    // 2. APP — business event (order lifecycle)
    // -------------------------------------------------------------------------
    println!("--- 2. app_log! ---");
    app_log!(Info,  "order.created",   { order_id: 9001u64, customer_id: 5u64, total_idr: 350_000u64 });
    app_log!(Info,  "payment.captured", { order_id: 9001u64, method: "gopay", amount_idr: 350_000u64 });
    app_log!(Info,  "order.shipped",   { order_id: 9001u64, courier: "jne", tracking: "JNE123456" });
    app_log!(Warn,  "order.sla_breach", { order_id: 9001u64, sla_minutes: 30u32, elapsed_minutes: 45u32 });

    // -------------------------------------------------------------------------
    // 3. AI — LLM chat completion
    // -------------------------------------------------------------------------
    println!("--- 3. ai_log! ---");
    ai_log!(Info, AiPayload {
        provider_hash:   register_str("openai"),
        model_hash:      register_str("gpt-4o-mini"),
        input_tokens:    280,
        output_tokens:   420,
        latency_us:      950_000, // 950ms
        cost_micro_usd:  120,
        op_type:         0,    // chat
        streaming:       0,
        retries:         0,
        cache_hit:       0,    // miss
        provider_status: 200,
        ..AiPayload::default()
    });

    // Semantic cache hit — fast path
    ai_log!(Debug, AiPayload {
        provider_hash:   register_str("openai"),
        model_hash:      register_str("gpt-4o-mini"),
        input_tokens:    280,
        output_tokens:   420,
        latency_us:      800,  // sub-ms from cache
        cost_micro_usd:  0,
        op_type:         0,
        cache_hit:       2,    // semantic hit
        provider_status: 0,
        ..AiPayload::default()
    });

    // -------------------------------------------------------------------------
    // 4. DB — database queries
    // -------------------------------------------------------------------------
    println!("--- 4. db_log! ---");
    db_log!(Info, DbPayload {
        db_hash:       register_str("postgres"),
        table_hash:    register_str("orders"),
        query_hash:    register_str("INSERT INTO orders (customer_id, total) VALUES ($1, $2)"),
        duration_us:   1_200,
        rows_affected: 1,
        op_type:       1, // INSERT
        prepared:      1,
        tx_state:      1, // begin
        error_code:    0,
        pool_id:       0,
        shard_id:      0,
        ..DbPayload::default()
    });

    db_log!(Warn, DbPayload {
        db_hash:       register_str("postgres"),
        table_hash:    register_str("inventory"),
        query_hash:    register_str("SELECT stock FROM inventory WHERE sku = $1 FOR UPDATE"),
        duration_us:   85_000, // slow query
        rows_affected: 1,
        op_type:       0, // SELECT
        prepared:      1,
        tx_state:      0,
        error_code:    0,
        ..DbPayload::default()
    });

    // -------------------------------------------------------------------------
    // 5. MQ — message queue events
    // -------------------------------------------------------------------------
    println!("--- 5. mq_log! ---");
    mq_log!(Info, MqPayload {
        broker_hash:    register_str("kafka"),
        topic_hash:     register_str("order.events"),
        group_hash:     register_str("order-fulfillment"),
        offset:         204_892,
        message_bytes:  384,
        e2e_latency_us: 2_100,
        op_type:        0,    // publish
        partition:      3,
        retries:        0,
        compression:    2,    // lz4
        ..MqPayload::default()
    });

    mq_log!(Warn, MqPayload {
        broker_hash:    register_str("kafka"),
        topic_hash:     register_str("payment.events"),
        group_hash:     register_str("payment-processor"),
        offset:         18_443,
        message_bytes:  256,
        e2e_latency_us: 25_000,
        op_type:        4,    // dlq
        partition:      1,
        retries:        5,
        ..MqPayload::default()
    });

    // -------------------------------------------------------------------------
    // 6. SYSTEM — OS resource snapshot
    // -------------------------------------------------------------------------
    println!("--- 6. system_log! ---");
    system_log!(Info, SystemPayload {
        cpu_pct_x100:      4_250,       // 42.50%
        mem_kb:            2_048_000,   // ~2 GB used
        mem_avail_kb:      6_144_000,   // ~6 GB free
        fd_count:          1_024,
        thread_count:      48,
        socket_count:      312,
        event_type:        0,           // metrics
        signal_num:        0,
        exit_code:         0,
        disk_read_bytes:   4_096,
        disk_write_bytes:  16_384,
        net_rx_bytes:      1_048_576,
        net_tx_bytes:      524_288,
        ..SystemPayload::default()
    });

    system_log!(Warn, SystemPayload {
        cpu_pct_x100:  9_800, // 98.00% — high CPU
        mem_kb:        7_800_000,
        mem_avail_kb:  196_000,
        fd_count:      60_000, // near limit
        thread_count:  512,
        socket_count:  4_096,
        event_type:    0,
        ..SystemPayload::default()
    });

    // -------------------------------------------------------------------------
    // 7. SECURITY — auth and authz events
    // -------------------------------------------------------------------------
    println!("--- 7. security_log! ---");
    security_log!(Info, SecurityPayload {
        actor_hash:      register_str("user:5"),
        resource_hash:   register_str("/api/v1/orders"),
        action_hash:     register_str("POST"),
        client_ip:       0xC0A8_0101, // 192.168.1.1
        event_type:      0,  // auth
        outcome:         0,  // allow
        risk_score:      10,
        mfa_factor:      1,  // totp
        session_id:      0xDEAD_BEEF_CAFE_0001,
        failed_attempts: 0,
        geo_region:      360, // Indonesia
        ..SecurityPayload::default()
    });

    // Brute-force attempt from anomalous IP
    security_log!(Error, SecurityPayload {
        actor_hash:      register_str("user:unknown"),
        resource_hash:   register_str("/api/v1/auth/login"),
        action_hash:     register_str("POST"),
        client_ip:       0xDE_AD_BE_EF,
        event_type:      3,  // anomaly
        outcome:         1,  // deny
        risk_score:      230,
        mfa_factor:      0,
        failed_attempts: 15,
        geo_region:      0,  // unknown
        ..SecurityPayload::default()
    });

    tokio::time::sleep(std::time::Duration::from_millis(500)).await;
    println!("\n=== All 7 log categories demonstrated ===");
}
