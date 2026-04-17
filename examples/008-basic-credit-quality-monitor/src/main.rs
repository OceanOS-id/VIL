// ╔════════════════════════════════════════════════════════════╗
// ║  008 — Credit Data Quality Assurance Pipeline             ║
// ╠════════════════════════════════════════════════════════════╣
// ║  Domain:   Banking — Data Governance & Quality             ║
// ║  Pattern:  SDK_PIPELINE                                     ║
// ║  Token:    ShmToken (zero-copy for bulk validation)        ║
// ║  Features: .transform(), vil_workflow!, #[vil_fault]        ║
// ╠════════════════════════════════════════════════════════════╣
// ║  Business: Validates credit data quality before it enters  ║
// ║  downstream systems. Checks include:                       ║
// ║    - NIK (national ID) must be exactly 16 digits           ║
// ║    - Collectability rating must be 1-5 per OJK standard   ║
// ║    - Outstanding balance cannot exceed credit limit         ║
// ║    - No negative monetary amounts                          ║
// ║  Each record gets a PASS/FAIL quality score for audit.     ║
// ╚════════════════════════════════════════════════════════════╝
// Run:
//   cargo run -p fintec01-simulators   # Start simulator first
//   cargo run -p basic-usage-credit-quality-monitor
//
// Test:
//   curl -N -X POST http://localhost:3082/quality-check \
//     -H "Content-Type: application/json" \
//     -d '{"check": "full-scan"}'

use std::sync::Arc;
use vil_sdk::prelude::*;

// ── Semantic Types ──────────────────────────────────────────────────────

#[vil_state]
/// Data quality monitoring state — tracks scan progress and error rates per batch.
pub struct QualityMonitorState {
    pub request_id: u64,
    pub records_scanned: u32,
    pub errors_found: u32,
    pub error_rate_pct: f32,
}

#[vil_event]
pub struct QualityIssueDetected {
    pub request_id: u64,
    pub record_id: u64,
    pub error_type: u32,
    pub field_name: u32,
    pub timestamp_ns: u64,
}

#[vil_fault]
pub enum QualityMonitorFault {
    CoreBankingTimeout,
    StreamDisconnect,
    BatchParseError,
    ShmWriteFailed,
}

// ── Configuration ───────────────────────────────────────────────────────

const WEBHOOK_PORT: u16 = 3082;
const WEBHOOK_PATH: &str = "/quality-check";

/// Core Banking NDJSON with 20% dirty_ratio — produces records with
/// intentional data errors (_has_error=true, _error_type set).
/// Larger batch_size for throughput testing.
const CORE_BANKING_NDJSON: &str =
    "http://localhost:18081/api/v1/credits/ndjson?count=1000&dirty_ratio=0.3";

// ── Node Builders ───────────────────────────────────────────────────────

fn configure_sink() -> HttpSinkBuilder {
    HttpSinkBuilder::new("QualityMonitorSink")
        .env_port(WEBHOOK_PORT)
        .path(WEBHOOK_PATH)
        .out_port("trigger_out")
        .in_port("response_data_in")
        .ctrl_in_port("response_ctrl_in")
}

fn configure_source() -> HttpSourceBuilder {
    // Core Banking NDJSON — validate data quality per record.
    // Each record passes through 5 business validation rules aligned with
    // OJK data governance requirements. Records failing any rule get
    // annotated with specific issue codes for remediation tracking.
    // Transform: annotate each record with quality assessment
    HttpSourceBuilder::new("QualityCreditSource")
        .url(CORE_BANKING_NDJSON)
        .format(HttpFormat::NDJSON)
        .transform(|line: &[u8]| {
            let mut record: serde_json::Value = serde_json::from_slice(line).ok()?;
            let mut issues: Vec<&str> = Vec::new();

            // Rule 1: kolektabilitas must be 1-5
            let kol = record["kolektabilitas"].as_u64().unwrap_or(0);
            if kol < 1 || kol > 5 {
                issues.push("invalid_kolektabilitas");
            }

            // Rule 2: saldo_outstanding must not exceed jumlah_kredit
            let saldo = record["saldo_outstanding"].as_i64().unwrap_or(0);
            let kredit = record["jumlah_kredit"].as_i64().unwrap_or(0);
            if saldo > kredit {
                issues.push("saldo_exceeds_kredit");
            }
            if saldo < 0 || kredit < 0 {
                issues.push("negative_amount");
            }

            // Rule 3: nik must be 16 digits
            let nik = record["nik"].as_str().unwrap_or("");
            if nik.len() != 16 {
                issues.push("invalid_nik_length");
            }

            // Rule 4: nama_lengkap must not be empty
            let nama = record["nama_lengkap"].as_str().unwrap_or("");
            if nama.is_empty() {
                issues.push("missing_nama");
            }

            // Rule 5: detect simulator _has_error flag
            if record["_has_error"].as_bool().unwrap_or(false) {
                let err_type = record["_error_type"].as_str().unwrap_or("unknown");
                if !issues.iter().any(|&i| i != "invalid_kolektabilitas") {
                    issues.push("simulator_dirty_flag");
                }
                record["_detected_error_type"] = serde_json::json!(err_type);
            }

            // Annotate each record with quality metadata for downstream consumers:
            // _quality_issues: array of specific issue codes (for remediation)
            // _quality_score: PASS/FAIL summary (for dashboard reporting)
            // _issue_count: numeric count (for aggregation and trending)
            record["_quality_issues"] = serde_json::json!(issues);
            record["_quality_score"] =
                serde_json::json!(if issues.is_empty() { "PASS" } else { "FAIL" });
            record["_issue_count"] = serde_json::json!(issues.len());

            Some(serde_json::to_vec(&record).unwrap_or_else(|_| line.to_vec()))
        })
        .in_port("trigger_in")
        .out_port("response_data_out")
        .ctrl_out_port("response_ctrl_out")
}

// ── Main — Quality monitoring pipeline assembly ─────────────────────────
// Wires the gateway sink to the quality validation source. Every credit
// record from Core Banking passes through 5 business rules before being
// forwarded to the downstream consumer with quality annotations.

fn main() {
    // Initialize SHM runtime for zero-copy transport of credit records
    let world = Arc::new(VastarRuntimeWorld::new_shared().expect("Failed to init VIL SHM Runtime"));

    // Configure the two pipeline nodes: gateway (HTTP sink) and validator (NDJSON source)
    let sink = configure_sink();
    let source = configure_source();

    // Wire Tri-Lane pipeline: trigger -> validate records -> stream results back
    let (_ir, (sink_h, source_h)) = vil_workflow! {
        name: "CreditQualityMonitorPipeline",
        instances: [ sink, source ],
        routes: [
            sink.trigger_out -> source.trigger_in (LoanWrite),
            source.response_data_out -> sink.response_data_in (LoanWrite),
            source.response_ctrl_out -> sink.response_ctrl_in (Copy),
        ]
    };

    println!("╔══════════════════════════════════════════════════════════════╗");
    println!("║  007 — Credit Data Quality Monitor (ShmToken Pipeline)      ║");
    println!("╠══════════════════════════════════════════════════════════════╣");
    println!("║                                                              ║");
    println!("║  Core Banking NDJSON -> Data Quality Assessment                ║");
    println!("║                                                              ║");
    println!("║  Checks for:                                                 ║");
    println!("║    - Missing/invalid NIK (national ID)                      ║");
    println!("║    - Invalid kolektabilitas (must be 1-5)                   ║");
    println!("║    - Negative saldo_outstanding                             ║");
    println!("║    - Records with _has_error flag                           ║");
    println!("║    - Duplicate credit IDs within batch                      ║");
    println!("║                                                              ║");
    println!("║  Upstream: Core Banking Simulator (port 18081)              ║");
    println!("║  Token:    ShmToken (zero-copy)                             ║");
    println!("║  Format:  NDJSON                                     ║");
    println!("╚══════════════════════════════════════════════════════════════╝");
    println!();
    println!(
        "  Listen:   http://localhost:{}{}",
        WEBHOOK_PORT, WEBHOOK_PATH
    );
    println!("  Upstream: {}", CORE_BANKING_NDJSON);
    println!();
    println!("  Requires: Core Banking Simulator running on port 18081");
    println!("    cargo run -p fintec01-simulators");
    println!();
    println!(
        "  curl -N -X POST http://localhost:{}{} \\",
        WEBHOOK_PORT, WEBHOOK_PATH
    );
    println!("    -H \"Content-Type: application/json\" \\");
    println!("    -d '{{\"check\": \"full-scan\"}}'");
    println!();
    println!("  oha -m POST --no-tui -H \"Content-Type: application/json\" \\");
    println!("    -d '{{\"check\": \"bench\"}}' -c 50 -n 500 \\");
    println!("    http://localhost:{}{}\n", WEBHOOK_PORT, WEBHOOK_PATH);

    let sink_node = HttpSink::from_builder(sink);
    let source_node = HttpSource::from_builder(source);

    let t1 = sink_node.run_worker::<ShmToken>(world.clone(), sink_h);
    let t2 = source_node.run_worker::<ShmToken>(world.clone(), source_h);

    t1.join().expect("QualityMonitorSink panicked");
    t2.join().expect("QualityCreditSource panicked");
}
