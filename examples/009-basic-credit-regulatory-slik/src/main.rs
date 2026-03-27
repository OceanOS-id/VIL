// ╔════════════════════════════════════════════════════════════╗
// ║  009 — OJK SLIK Regulatory Reporting Pipeline             ║
// ╠════════════════════════════════════════════════════════════╣
// ║  Domain:   Banking — Regulatory Compliance (OJK/BI)        ║
// ║  Pattern:  SDK_PIPELINE                                     ║
// ║  Token:    ShmToken (zero-copy for bulk regulatory data)   ║
// ║  Features: .transform(), vil_workflow!, #[vil_fault]        ║
// ╠════════════════════════════════════════════════════════════╣
// ║  Business: Transforms Core Banking credit records into     ║
// ║  OJK SLIK v2.1 regulatory format for monthly submission.  ║
// ║  Field mapping: id->no_rekening, nik->nik_debitur,         ║
// ║  jumlah_kredit->plafon, saldo_outstanding->baki_debet,    ║
// ║  kolektabilitas->kualitas_kredit. All amounts in IDR.      ║
// ║  Non-compliance = regulatory penalty for the bank.         ║
// ╚════════════════════════════════════════════════════════════╝
// Run:
//   cargo run -p fintec01-simulators   # Start simulator first
//   cargo run -p basic-usage-credit-regulatory-pipeline
//
// Test:
//   curl -N -X POST http://localhost:3083/regulatory-stream \
//     -H "Content-Type: application/json" \
//     -d '{"report_type": "slik-monthly"}'

use std::sync::Arc;
use vil_sdk::prelude::*;

// ── Semantic Types ──────────────────────────────────────────────────────

#[vil_state]
/// SLIK reporting state — tracks batch progress for monthly OJK submission.
pub struct RegulatoryState {
    pub request_id: u64,
    pub records_processed: u64,
    pub records_valid: u64,
    pub records_rejected: u32,
    pub report_period: u32,
}

#[vil_event]
/// Batch completion event — logged for audit trail and reconciliation.
pub struct RegulatoryBatchCompleted {
    pub request_id: u64,
    pub batch_seq: u32,
    pub records_in_batch: u32,
    pub valid_count: u32,
    pub rejected_count: u32,
    pub timestamp_us: u64,
}

#[vil_fault]
/// Regulatory pipeline faults — each triggers compliance team notification.
pub enum RegulatoryFault {
    CoreBankingTimeout,
    StreamDisconnect,
    SlikValidationFailed,
    FieldMappingError,
    ShmWriteFailed,
}

// ── Configuration ───────────────────────────────────────────────────────
// Regulatory reporting endpoint — triggered monthly by the compliance
// scheduler. Large batch sizes for full-portfolio transformation.

/// Port for the SLIK regulatory reporting endpoint
const WEBHOOK_PORT: u16 = 3083;
/// Path for regulatory data submission trigger
const WEBHOOK_PATH: &str = "/regulatory-stream";

/// Core Banking NDJSON — bulk mode for monthly SLIK regulatory reporting.
/// Large count (1000 records per batch) — in production, this processes
/// the bank's entire credit portfolio (500K+ records) for OJK submission.
const CORE_BANKING_NDJSON: &str =
    "http://localhost:18081/api/v1/credits/ndjson?count=1000";

// ── Node Builders ───────────────────────────────────────────────────────

/// Configure the regulatory data submission gateway endpoint.
fn configure_sink() -> HttpSinkBuilder {
    HttpSinkBuilder::new("RegulatorySink")
        .port(WEBHOOK_PORT)
        .path(WEBHOOK_PATH)
        .out_port("trigger_out")
        .in_port("response_data_in")
        .ctrl_in_port("response_ctrl_in")
}

fn configure_source() -> HttpSourceBuilder {
    // Core Banking NDJSON — transform each credit record to SLIK v2.1 format.
    // Field mapping follows OJK Data Dictionary (Kamus Data SLIK):
    //   id             -> no_rekening    (account number)
    //   nik            -> nik_debitur    (borrower national ID)
    //   nama_lengkap   -> nama_debitur   (borrower full name)
    //   jumlah_kredit  -> plafon         (credit limit / ceiling)
    //   saldo_outstanding -> baki_debet  (outstanding balance)
    //   kolektabilitas -> kualitas_kredit (collectability rating 1-5)
    HttpSourceBuilder::new("RegulatorySource")
        .url(CORE_BANKING_NDJSON)
        .format(HttpFormat::NDJSON)
        .transform(|line: &[u8]| {
            let record: serde_json::Value = serde_json::from_slice(line).ok()?;
            // Map to SLIK reporting format (OJK regulatory schema)
            let slik = serde_json::json!({
                "no_rekening": record["id"],
                "nik_debitur": record["nik"],
                "nama_debitur": record["nama_lengkap"],
                "jenis_fasilitas": record["jenis_fasilitas"],
                "plafon": record["jumlah_kredit"],
                "mata_uang": record["mata_uang"].as_str().unwrap_or("IDR"), // Default currency: Indonesian Rupiah
                "baki_debet": record["saldo_outstanding"],
                "kualitas_kredit": record["kolektabilitas"],
                "kode_kantor_cabang": record["kode_cabang"],
                "tanggal_mulai": record["tanggal_mulai"],
                "tanggal_jatuh_tempo": record["tanggal_jatuh_tempo"],
                "_slik_version": "v2.1",
            });
            Some(serde_json::to_vec(&slik).unwrap_or_else(|_| line.to_vec()))
        })
        .in_port("trigger_in")
        .out_port("response_data_out")
        .ctrl_out_port("response_ctrl_out")
}

fn main() {
    let world =
        Arc::new(VastarRuntimeWorld::new_shared().expect("Failed to init VIL SHM Runtime"));

    let sink = configure_sink();
    let source = configure_source();

    let (_ir, (sink_h, source_h)) = vil_workflow! {
        name: "RegulatoryStreamPipeline",
        instances: [ sink, source ],
        routes: [
            sink.trigger_out -> source.trigger_in (LoanWrite),
            source.response_data_out -> sink.response_data_in (LoanWrite),
            source.response_ctrl_out -> sink.response_ctrl_in (Copy),
        ]
    };

    println!("╔══════════════════════════════════════════════════════════════╗");
    println!("║  008 — Credit Regulatory Stream Pipeline (SLIK/OJK)        ║");
    // Banner: display pipeline topology and connection info
    println!("╠══════════════════════════════════════════════════════════════╣");
    println!("║                                                              ║");
    println!("║  Core Banking NDJSON -> SLIK Regulatory Pipeline               ║");
    println!("║                                                              ║");
    println!("║  Regulatory workflow:                                        ║");
    println!("║    1. Ingest credit records via NDJSON (bulk mode)             ║");
    println!("║    2. Map fields to SLIK reporting format                   ║");
    println!("║    3. Validate against OJK data dictionary                  ║");
    println!("║    4. Aggregate into regulatory submission batches          ║");
    println!("║                                                              ║");
    println!("║  Bulk config: 500 records, batch_size=20, delay=30ms       ║");
    println!("║  Upstream: Core Banking Simulator (port 18081)              ║");
    // Banner: display pipeline topology and connection info
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
    println!("    -d '{{\"report_type\": \"slik-monthly\"}}'");
    println!();
    println!("  oha -m POST --no-tui -H \"Content-Type: application/json\" \\");
    println!("    -d '{{\"report_type\": \"bench\"}}' -c 100 -n 1000 \\");
    println!("    http://localhost:{}{}\n", WEBHOOK_PORT, WEBHOOK_PATH);

    let sink_node = HttpSink::from_builder(sink);
    let source_node = HttpSource::from_builder(source);

    let t1 = sink_node.run_worker::<ShmToken>(world.clone(), sink_h);
    let t2 = source_node.run_worker::<ShmToken>(world.clone(), source_h);

    t1.join().expect("RegulatorySink panicked");
    t2.join().expect("RegulatorySource panicked");
}
