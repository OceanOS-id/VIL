// ╔════════════════════════════════════════════════════════════╗
// ║  005 — Core Banking Data Ingestion Pipeline               ║
// ╠════════════════════════════════════════════════════════════╣
// ║  Domain:   Banking — Core Banking System Integration       ║
// ║  Pattern:  SDK_PIPELINE                                     ║
// ║  Token:    ShmToken (zero-copy for high-throughput NDJSON) ║
// ║  Features: .transform(), vil_workflow!, #[vil_fault]        ║
// ╠════════════════════════════════════════════════════════════╣
// ║  Business: Ingests credit portfolio data from the Core     ║
// ║  Banking System via NDJSON streaming. Each record is       ║
// ║  enriched with risk category (OJK collectability) and      ║
// ║  LTV ratio for downstream analytics and reporting.         ║
// ║  Handles 1000+ records/second with zero-copy transport.    ║
// ╚════════════════════════════════════════════════════════════╝
// Core Banking Data Ingestion (Multi-Service Mesh, ShmToken)
// =============================================================================
//
// Two-node pipeline demonstrating VIL's Tri-Lane mesh with ShmToken
// against a real fintech NDJSON data source (Core Banking Simulator):
//
//   gateway (HttpSink :3084)
//     └──[Trigger, LoanWrite]──> ingest (HttpSource -> Core Banking NDJSON)
//        └──[Data, LoanWrite]──> gateway (streamed credit records)
//        └──[Control, Copy]────> gateway (stream completion signal)
//
// The Core Banking Simulator streams credit records via NDJSON at
// GET /api/v1/credits/ndjson on port 18081. Each line is one JSON record
// (id, nik, nama_lengkap, kolektabilitas, jumlah_kredit, saldo_outstanding).
//
// Run:
//   # Start Core Banking Simulator first:
//   cargo run -p fintec01-simulators
//
//   # Then run this example:
//   cargo run -p basic-usage-multiservice-mesh
//
// Test:
//   curl -N -X POST http://localhost:3084/ingest \
//     -H "Content-Type: application/json" \
//     -d '{"request":"stream-credits"}'
// =============================================================================

use std::sync::Arc;
use vil_sdk::prelude::*;

// ── Semantic Types ──────────────────────────────────────────────────────

#[vil_state]
pub struct MeshState {
    pub request_id: u64,
    pub batches_received: u32,
    pub records_ingested: u64,
    pub active_pipelines: u8,
}

#[vil_event]
pub struct CreditBatchIngested {
    pub request_id: u64,
    pub batch_size: u32,
    pub pipeline_id: u8,
    pub latency_us: u64,
}

#[vil_fault]
pub enum MeshFault {
    CoreBankingTimeout,
    SseStreamDisconnect,
    ShmWriteFailed,
    RouteNotFound,
}

// ── Configuration ───────────────────────────────────────────────────────

const GATEWAY_PORT: u16 = 3084;
const GATEWAY_PATH: &str = "/ingest";

/// Core Banking Simulator NDJSON endpoint.
/// Streams credit records as newline-delimited JSON (one record per line).
/// Query params: count, batch_size, delay_ms, dirty_ratio, seed
const CORE_BANKING_NDJSON: &str =
    "http://localhost:18081/api/v1/credits/ndjson?count=1000&dirty_ratio=0.3";

// ── Node Builders ───────────────────────────────────────────────────────

fn configure_gateway() -> HttpSinkBuilder {
    HttpSinkBuilder::new("Gateway")
        .port(GATEWAY_PORT)
        .path(GATEWAY_PATH)
        .out_port("trigger_out")
        .in_port("response_data_in")
        .ctrl_in_port("response_ctrl_in")
}

fn configure_credit_ingest() -> HttpSourceBuilder {
    // Core Banking NDJSON — enrich each record with risk category + LTV
    HttpSourceBuilder::new("CreditIngest")
        .url(CORE_BANKING_NDJSON)
        .format(HttpFormat::NDJSON)
        .transform(|line: &[u8]| {
            let mut record: serde_json::Value = serde_json::from_slice(line).ok()?;
            let kol = record["kolektabilitas"].as_u64().unwrap_or(0);
            record["_risk_category"] = serde_json::json!(match kol {
                1 => "LANCAR",
                2 => "DALAM_PERHATIAN_KHUSUS",
                3 => "KURANG_LANCAR",
                4 => "DIRAGUKAN",
                5 => "MACET",
                _ => "UNKNOWN",
            });
            let saldo = record["saldo_outstanding"].as_f64().unwrap_or(0.0);
            let plafon = record["jumlah_kredit"].as_f64().unwrap_or(1.0);
            record["_ltv_ratio"] = serde_json::json!((saldo / plafon * 100.0).round() / 100.0);
            Some(serde_json::to_vec(&record).unwrap_or_else(|_| line.to_vec()))
        })
        .in_port("trigger_in")
        .out_port("response_data_out")
        .ctrl_out_port("response_ctrl_out")
}

// ── Main ────────────────────────────────────────────────────────────────

fn main() {
    let world = Arc::new(VastarRuntimeWorld::new_shared().expect("Failed to init VIL SHM Runtime"));

    let gateway = configure_gateway();
    let ingest = configure_credit_ingest();

    // Wire pipeline: gateway <-> credit ingest (Tri-Lane SHM)
    let (_ir, (gateway_h, ingest_h)) = vil_workflow! {
        name: "MultiServiceMesh",
        instances: [ gateway, ingest ],
        routes: [
            gateway.trigger_out -> ingest.trigger_in (LoanWrite),
            ingest.response_data_out -> gateway.response_data_in (LoanWrite),
            ingest.response_ctrl_out -> gateway.response_ctrl_in (Copy),
        ]
    };

    // Banner
    println!("╔══════════════════════════════════════════════════════════════╗");
    println!("╔══════════════════════════════════════════════════════════════╗");
    println!("║  004 — Multi-Service Mesh (Core Banking NDJSON)            ║");
    println!("╠══════════════════════════════════════════════════════════════╣");
    println!("║                                                              ║");
    println!("║  gateway (:3084/ingest) ──[LoanWrite]──> CreditIngest      ║");
    println!("║                         <──[LoanWrite]── (NDJSON records)  ║");
    println!("║                         <──[Copy]─────── (stream complete) ║");
    println!("║                                                              ║");
    println!("║  Upstream: Core Banking Simulator (port 18081)              ║");
    println!("║  Token:    ShmToken (multi-pipeline, zero-copy)             ║");
    println!("║  Format:   NDJSON (newline-delimited JSON)                  ║");
    println!("╚══════════════════════════════════════════════════════════════╝");
    println!();
    println!("  Requires: Core Banking Simulator running on port 18081");
    println!("    cargo run -p fintec01-simulators");
    println!();
    println!("  Test:");
    println!(
        "  curl -N -X POST http://localhost:{}{} \\",
        GATEWAY_PORT, GATEWAY_PATH
    );
    println!("    -H \"Content-Type: application/json\" \\");
    println!("    -d '{{\"request\":\"stream-credits\"}}'");
    println!();
    println!("  Benchmark:");
    println!("  oha -m POST --no-tui -H \"Content-Type: application/json\" \\");
    println!("    -d '{{\"request\":\"bench\"}}' -c 50 -n 500 \\");
    println!("    http://localhost:{}{}", GATEWAY_PORT, GATEWAY_PATH);
    println!();

    // Spawn workers — ShmToken for multi-pipeline zero-copy
    let gw = HttpSink::from_builder(gateway);
    let ingest_node = HttpSource::from_builder(ingest);

    let t1 = gw.run_worker::<ShmToken>(world.clone(), gateway_h);
    let t2 = ingest_node.run_worker::<ShmToken>(world.clone(), ingest_h);

    t1.join().expect("Gateway worker panicked");
    t2.join().expect("CreditIngest worker panicked");
}
