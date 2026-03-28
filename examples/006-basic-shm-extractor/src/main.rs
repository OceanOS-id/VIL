// ╔════════════════════════════════════════════════════════════╗
// ║  006 — High-Frequency Trading Data Processor              ║
// ╠════════════════════════════════════════════════════════════╣
// ║  Domain:   Capital Markets — HFT Data Ingestion            ║
// ║  Pattern:  VX_APP                                           ║
// ║  Token:    N/A (HTTP server with SHM zero-copy)            ║
// ║  Features: ShmSlice, VilResponse, blocking_with            ║
// ╠════════════════════════════════════════════════════════════╣
// ║  Business: Processes high-frequency market data via SHM    ║
// ║  zero-copy extraction. ShmSlice eliminates buffer copies   ║
// ║  — critical for tick-by-tick price feeds where every       ║
// ║  microsecond matters. CPU-bound analytics (e.g., VWAP,     ║
// ║  order book reconstruction) run on blocking thread pool    ║
// ║  to avoid starving the async I/O executor.                 ║
// ╚════════════════════════════════════════════════════════════╝
// High-Frequency Trading Data Processor (VX Server Mode, SHM Zero-Copy)
// =============================================================================
//
// Demonstrates VIL's ShmSlice and ShmContext extractors using the
// VX Process-Oriented architecture (VilApp + ServiceProcess, GenericToken):
//
// NOTE: This is SERVER MODE (GenericToken). ShmSlice writes HTTP body to
// ExchangeHeap (1 copy), then handler reads from SHM (0 additional copies).
// This is NOT the same as ShmToken pipeline mode (true zero-copy Tri-Lane).
//
//   - POST /ingest      → ShmSlice extractor (request body → SHM, zero-copy read)
//   - POST /compute     → blocking_with for CPU-bound work on blocking thread pool
//   - GET  /shm-stats   → ShmContext to inspect ExchangeHeap region statistics
//   - GET  /benchmark   → simple throughput measurement endpoint
//
// VX highlights:
//   - ServiceProcess groups endpoints as a logical Process
//   - VilApp orchestrates processes with Tri-Lane mesh
//   - Handlers stay EXACTLY the same as classic vil-server
//
// How ShmSlice works:
//   1. HTTP request body (Bytes) arrives
//   2. Body is written into a pre-allocated ExchangeHeap region (1 copy)
//   3. ShmSlice holds region_id + offset + length
//   4. Handler reads data directly from SHM (0 additional copies)
//   5. Data can be forwarded to mesh services without copying
//
// Built-in endpoints (auto-provided by VilApp):
//   - GET /health     -> health check
//   - GET /ready      -> readiness probe
//   - GET /metrics    -> Prometheus-style metrics
//   - GET /info       -> server info
//
// Run:
//   cargo run -p basic-usage-shm-zerocopy
//
// Test:
//   # Ingest data via ShmSlice
//   curl -X POST http://localhost:8080/api/shm-demo/ingest \
//     -H 'Content-Type: application/octet-stream' \
//     -d 'Hello, SHM world!'
//
//   # Ingest JSON via ShmSlice
//   curl -X POST http://localhost:8080/api/shm-demo/ingest \
//     -H 'Content-Type: application/json' \
//     -d '{"sensor":"temp-01","value":23.5}'
//
//   # CPU-bound compute via blocking thread pool
//   curl -X POST http://localhost:8080/api/shm-demo/compute \
//     -H 'Content-Type: application/json' \
//     -d '{"iterations":1000000}'
//
//   # Inspect SHM region stats
//   curl http://localhost:8080/api/shm-demo/shm-stats
//
//   # Simple benchmark endpoint
//   curl http://localhost:8080/api/shm-demo/benchmark
//
//   # Built-in endpoints
//   curl http://localhost:8080/health
// =============================================================================

use vil_server::prelude::*;

// ---------------------------------------------------------------------------
// Typed response structs (VIL Way: no serde_json::json!)
// ---------------------------------------------------------------------------

#[derive(Serialize)]
struct IngestResponse {
    status: &'static str,
    bytes_received: usize,
    shm_region_id: String,
    preview: String,
    is_valid_json: bool,
    transport: &'static str,
    copies: &'static str,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct ComputeRequest {
    #[serde(default = "default_iterations")]
    iterations: u64,
}

fn default_iterations() -> u64 {
    100_000
}

#[derive(Serialize)]
struct ComputeResponse {
    status: &'static str,
    iterations: u64,
    result_hash: u64,
    elapsed_ms: f64,
    thread: &'static str,
    note: &'static str,
}

#[derive(Serialize)]
struct RegionInfo {
    region_id: String,
    capacity_bytes: usize,
    used_bytes: usize,
    remaining_bytes: usize,
    utilization_pct: String,
}

#[derive(Serialize)]
struct ShmStatsResponse {
    shm_available: bool,
    region_count: usize,
    regions: Vec<RegionInfo>,
    note: &'static str,
}

#[derive(Serialize)]
struct BenchmarkResponse {
    ok: bool,
    timestamp_ns: u64,
}

// ---------------------------------------------------------------------------
// POST /ingest — demonstrates ShmSlice extractor
// ---------------------------------------------------------------------------

/// Receives request body via ShmSlice (zero-copy after initial SHM write).
/// The body is automatically placed into an ExchangeHeap region. The handler
/// can read it without any further copies.
async fn ingest(body: ShmSlice) -> VilResponse<IngestResponse> {
    let len = body.len();
    let region_id = body.region_id();

    // Try to interpret as UTF-8 text for display
    let preview = match body.text() {
        Ok(text) => {
            let truncated: String = text.chars().take(100).collect();
            truncated
        }
        Err(_) => format!("<binary {} bytes>", len),
    };

    // Try to deserialize as JSON (zero-copy read from SHM)
    let is_json = body.json::<serde_json::Value>().is_ok();

    VilResponse::ok(IngestResponse {
        status: "ingested",
        bytes_received: len,
        shm_region_id: format!("{:?}", region_id),
        preview,
        is_valid_json: is_json,
        transport: "SHM zero-copy",
        copies: "1 (kernel → SHM), then 0 for handler read",
    })
}

// ---------------------------------------------------------------------------
// POST /compute — demonstrates blocking_with for CPU-bound handlers
// ---------------------------------------------------------------------------

/// CPU-bound compute handler. Uses blocking_with to run on Tokio's blocking
/// thread pool, preventing starvation of the async executor.
async fn compute(body: ShmSlice) -> vil_server::axum::response::Response {
    let req: ComputeRequest = body.json().expect("invalid JSON body");
    let iterations = req.iterations.min(100_000_000); // cap at 100M

    blocking_with(move || {
        let start = std::time::Instant::now();

        // Simulate CPU-bound work (e.g., ML inference, crypto, compression)
        let mut result: u64 = 0;
        for i in 0..iterations {
            result = result.wrapping_add(i.wrapping_mul(17).wrapping_add(31));
        }

        let elapsed = start.elapsed();

        VilResponse::ok(ComputeResponse {
            status: "computed",
            iterations,
            result_hash: result,
            elapsed_ms: elapsed.as_secs_f64() * 1000.0,
            thread: "blocking_pool (not async executor)",
            note: "CPU-bound work runs on spawn_blocking, freeing async threads for I/O",
        })
    })
    .await
}

// ---------------------------------------------------------------------------
// GET /shm-stats — demonstrates ShmContext extractor
// ---------------------------------------------------------------------------

/// Shows ExchangeHeap region statistics via the ShmContext extractor.
/// ShmContext is automatically extracted from AppState and provides
/// read-only access to SHM metadata.
async fn shm_stats(shm: ShmContext) -> VilResponse<ShmStatsResponse> {
    let regions: Vec<RegionInfo> = shm
        .region_stats()
        .iter()
        .map(|s| {
            let utilization = if s.capacity > 0 {
                (s.used as f64 / s.capacity as f64) * 100.0
            } else {
                0.0
            };
            RegionInfo {
                region_id: format!("{:?}", s.region_id),
                capacity_bytes: s.capacity,
                used_bytes: s.used,
                remaining_bytes: s.remaining,
                utilization_pct: format!("{:.1}", utilization),
            }
        })
        .collect();

    VilResponse::ok(ShmStatsResponse {
        shm_available: shm.available,
        region_count: shm.region_count(),
        regions,
        note: "Regions are created on-demand by ShmSlice and ShmResponse",
    })
}

// ---------------------------------------------------------------------------
// GET /benchmark — simple throughput test endpoint
// ---------------------------------------------------------------------------

/// Minimal-overhead endpoint for benchmarking request throughput.
/// Returns a small JSON payload with timing info. Use with wrk/ab/hey:
///   hey -n 10000 -c 50 http://localhost:8080/benchmark
async fn benchmark() -> VilResponse<BenchmarkResponse> {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();

    VilResponse::ok(BenchmarkResponse {
        ok: true,
        timestamp_ns: now.as_nanos() as u64,
    })
}

// ---------------------------------------------------------------------------
// Main
// ---------------------------------------------------------------------------

#[tokio::main]
async fn main() {
    // VX: Define service as a Process
    let shm_service = ServiceProcess::new("shm-demo")
        .endpoint(Method::POST, "/ingest", post(ingest))
        .endpoint(Method::POST, "/compute", post(compute))
        .endpoint(Method::GET, "/shm-stats", get(shm_stats))
        .endpoint(Method::GET, "/benchmark", get(benchmark));

    // VX: Run as Process-Oriented app
    VilApp::new("shm-extractor-demo")
        .port(8080)
        .observer(true)
        .service(shm_service)
        .run()
        .await;
}
