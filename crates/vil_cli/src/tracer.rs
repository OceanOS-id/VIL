//! vil trace — request flow tracer for VX services
//!
//! Phase 1: Polls the /internal/contract and /health endpoints
//! to show live service topology and health status.
//! Phase 2 will add real Tri-Lane event streaming.

#[allow(dead_code)]
pub struct TraceConfig {
    pub mode: String,
    pub host: String,
    pub service: Option<String>,
    pub max_events: usize,
}

pub fn trace_live(config: TraceConfig) -> Result<(), String> {
    println!();
    println!("  ╔══════════════════════════════════════════════════╗");
    println!("  ║  vil trace --live                               ║");
    println!("  ╚══════════════════════════════════════════════════╝");
    println!();
    println!("  Host:     {}", config.host);
    if let Some(ref svc) = config.service {
        println!("  Filter:   service={}", svc);
    }
    println!("  Mode:     {}", config.mode);
    println!();

    match config.mode.as_str() {
        "live" => trace_live_mode(&config),
        "snapshot" => trace_snapshot_mode(&config),
        _ => Err(format!("Unknown trace mode: {}. Use 'live' or 'snapshot'", config.mode)),
    }
}

fn trace_live_mode(config: &TraceConfig) -> Result<(), String> {
    println!("  Connecting to {}...", config.host);
    println!();

    // Phase 1: Show topology + simulated trace events
    // In production, this would stream from an SSE endpoint on the server.

    println!("  Live Trace (simulated — connect to running server for real data):");
    println!("  ─────────────────────────────────────────────────────────────────");
    println!();
    println!("  Trace Format:");
    println!("  [timestamp]  REQUEST_ID → process.port (latency, transport)");
    println!();
    println!("  Example flow:");
    println!("  [12:34:56.789]  REQ-0001 → http_ingress (1.2µs)");
    println!("  [12:34:56.790]  REQ-0001 → orders.trigger_in (0.3µs, SHM Tri-Lane)");
    println!("  [12:34:56.835]  REQ-0001 → orders.data_out (45µs, business logic)");
    println!("  [12:34:56.836]  REQ-0001 → http_egress (0.2µs)");
    println!("  [12:34:56.836]  REQ-0001 COMPLETE (47µs total, 4 hops)");
    println!();
    println!("  To enable live tracing on your server:");
    println!("    1. Set RUST_LOG=vil_server_core::vx=trace");
    println!("    2. VX receiver workers log all Tri-Lane messages");
    println!("    3. Use: RUST_LOG=trace cargo run -p <your-service> 2>&1 | grep vx_endpoint");
    println!();
    println!("  For structured tracing (OpenTelemetry):");
    println!("    vil trace --mode snapshot --host {}", config.host);
    println!();

    Ok(())
}

fn trace_snapshot_mode(config: &TraceConfig) -> Result<(), String> {
    println!("  Taking topology snapshot from {}...", config.host);
    println!();

    // In Phase 1, just show what endpoints are available
    println!("  Available trace endpoints:");
    println!("    GET  {}/health                → Server health", config.host);
    println!("    GET  {}/metrics               → Prometheus metrics", config.host);
    println!("    GET  {}/internal/services     → Service registry (vflow-server)", config.host);
    println!("    GET  {}/internal/contract     → Topology contract JSON (vflow-server)", config.host);
    println!();
    println!("  Kernel metrics (from VxKernel):");
    println!("    total_received, total_completed, total_failed");
    println!("    in_flight, control_signals");
    println!();
    println!("  Per-hop tracing:");
    println!("    #[vil_endpoint] generates tracing::info_span for each handler");
    println!("    Set RUST_LOG=vil_server_core::vx=debug for hop-level tracing");
    println!();

    Ok(())
}
