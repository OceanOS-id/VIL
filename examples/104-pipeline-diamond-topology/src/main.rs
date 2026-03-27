// ╔════════════════════════════════════════════════════════════╗
// ║  104 — Credit Report: Summary + Detail Views (Diamond)    ║
// ╠════════════════════════════════════════════════════════════╣
// ║  Domain:   Banking — Multi-View Credit Reporting           ║
// ║  Pattern:  MULTI_PIPELINE (Diamond Topology)              ║
// ║  Token:    ShmToken (shared ExchangeHeap)                 ║
// ║  Nodes:    4 (2 pipelines x 2 nodes each)                 ║
// ╠════════════════════════════════════════════════════════════╣
// ║  Business: Produces two views of the same credit data:    ║
// ║    Pipeline A (Summary): NPL-only compact view for        ║
// ║      executive dashboard — id, borrower, kol, balance     ║
// ║    Pipeline B (Detail): Full enrichment for risk analysts ║
// ║      — risk_score, LTV ratio, aging bucket, risk class    ║
// ║  Same source data, different transforms — the classic     ║
// ║  diamond pattern for multi-consumer reporting.             ║
// ╚════════════════════════════════════════════════════════════╝
//
// Demonstrates the Diamond Topology: one logical data source splits
// into two parallel processing paths, each producing a different
// view of the same data:
//
//                    Core Banking (:18081)
//                     /              \
//           Pipeline A              Pipeline B
//         (NPL Summary)         (Full Enrichment)
//         :3095/diamond        :3096/diamond-detail
//                     \              /
//                      Client (gather)
//
// Pipeline A: Extracts only NPL-relevant fields (summary view)
//   - Filters to kol >= 3 only, emits compact summary
//
// Pipeline B: Full enrichment with all computed fields (detail view)
//   - All records, with risk_score, ltv_ratio, risk_class, aging bucket
//
// Client hits either endpoint depending on the view needed.
//
// Run:
//   cargo run -p fintec01-simulators
//   cargo run -p 104-pipeline-diamond-topology
//
// Test:
//   curl -N -X POST http://localhost:3095/diamond \
//     -H "Content-Type: application/json" -d '{"request":"summary"}'
//   curl -N -X POST http://localhost:3096/diamond-detail \
//     -H "Content-Type: application/json" -d '{"request":"detail"}'

use std::sync::Arc;
use vil_sdk::prelude::*;

// ── Semantic Types ──────────────────────────────────────────────────────

/// State for the diamond topology dual-view processing.
#[vil_state]
pub struct DiamondState {
    pub request_id: u64,
    pub summary_records: u64,
    pub detail_records: u64,
    pub parallel_active: bool,
}

/// Emitted when a diamond branch completes processing.
#[vil_event]
pub struct DiamondBranchCompleted {
    pub branch_name: u8,
    pub record_count: u64,
    pub latency_us: u64,
    pub view_type: u8,
}

/// Faults for the diamond topology.
#[vil_fault]
pub enum DiamondFault {
    UpstreamTimeout,
    SummaryTransformError,
    DetailTransformError,
    ShmHeapExhausted,
    BranchDivergence,
}

// ── Configuration ───────────────────────────────────────────────────────

const SUMMARY_PORT: u16 = 3095;
const SUMMARY_PATH: &str = "/diamond";
const DETAIL_PORT: u16 = 3096;
const DETAIL_PATH: &str = "/diamond-detail";
/// Core Banking NDJSON endpoint — same data processed by summary + detail views.
const NDJSON_URL: &str = "http://localhost:18081/api/v1/credits/ndjson?count=100";

// ── Pipeline A: NPL Summary View ────────────────────────────────────────

/// Summary gateway — executive dashboard endpoint for NPL-only compact view.
fn configure_summary_sink() -> HttpSinkBuilder {
    HttpSinkBuilder::new("SummarySink")
        .port(SUMMARY_PORT)
        .path(SUMMARY_PATH)
        .out_port("trigger_out")
        .in_port("response_data_in")
        .ctrl_in_port("response_ctrl_in")
}

fn configure_summary_source() -> HttpSourceBuilder {
    HttpSourceBuilder::new("SummarySource")
        .url(NDJSON_URL)
        .format(HttpFormat::NDJSON)
        .transform(|line: &[u8]| {
            let record: serde_json::Value = serde_json::from_slice(line).ok()?;
            let kol = record["kolektabilitas"].as_u64().unwrap_or(0);

            // Summary view: only NPL records (kol >= 3), compact fields
            if kol >= 3 {
                let summary = serde_json::json!({
                    "id": record["id"],
                    "nik": record["nik"],
                    "nama": record["nama_lengkap"],
                    "kol": kol,
                    "saldo": record["saldo_outstanding"],
                    "_view": "SUMMARY",
                    "_npl_class": match kol {
                        3 => "KURANG_LANCAR",
                        4 => "DIRAGUKAN",
                        5 => "MACET",
                        _ => "NPL_OTHER",
                    },
                });
                Some(serde_json::to_vec(&summary).unwrap_or_default())
            } else {
                None // Drop healthy records in summary view
            }
        })
        .in_port("trigger_in")
        .out_port("response_data_out")
        .ctrl_out_port("response_ctrl_out")
}

// ── Pipeline B: Full Enrichment Detail View ─────────────────────────────
// For risk analysts who need complete credit assessment with computed metrics:
// risk_score, LTV ratio, risk class, risk category, and aging bucket (DPD).
// All records included (not just NPL) for comprehensive portfolio analysis.

/// Detail gateway — risk analyst endpoint for comprehensive enriched records.
fn configure_detail_sink() -> HttpSinkBuilder {
    HttpSinkBuilder::new("DetailSink")
        .port(DETAIL_PORT)
        .path(DETAIL_PATH)
        .out_port("trigger_out")
        .in_port("response_data_in")
        .ctrl_in_port("response_ctrl_in")
}

fn configure_detail_source() -> HttpSourceBuilder {
    HttpSourceBuilder::new("DetailSource")
        .url(NDJSON_URL)
        .format(HttpFormat::NDJSON)
        .transform(|line: &[u8]| {
            let mut record: serde_json::Value = serde_json::from_slice(line).ok()?;

            record["_view"] = serde_json::json!("DETAIL");

            // Risk score: kol * 20 + saldo / 1_000_000
            let kol = record["kolektabilitas"].as_u64().unwrap_or(0);
            let saldo = record["saldo_outstanding"].as_f64().unwrap_or(0.0);
            let plafon = record["jumlah_kredit"].as_f64().unwrap_or(1.0);
            let risk_score = kol as f64 * 20.0 + saldo / 1_000_000.0;

            record["_risk_score"] = serde_json::json!((risk_score * 100.0).round() / 100.0);
            record["_risk_class"] = serde_json::json!(
                if risk_score > 100.0 { "HIGH" }
                else if risk_score > 50.0 { "MEDIUM" }
                else { "LOW" }
            );

            // LTV ratio
            record["_ltv_ratio"] = serde_json::json!(
                ((saldo / plafon * 100.0).round() / 100.0)
            );

            // Risk category
            record["_risk_category"] = serde_json::json!(match kol {
                1 => "LANCAR",
                2 => "DALAM_PERHATIAN_KHUSUS",
                3 => "KURANG_LANCAR",
                4 => "DIRAGUKAN",
                5 => "MACET",
                _ => "UNKNOWN",
            });

            // DPD = Days Past Due — standard banking metric for loan delinquency aging
            // Aging bucket (based on kolektabilitas as proxy)
            record["_aging_bucket"] = serde_json::json!(match kol {
                1 => "0-30 DPD",
                2 => "31-60 DPD",
                3 => "61-90 DPD",
                4 => "91-120 DPD",
                5 => "120+ DPD",
                _ => "UNKNOWN",
            });

            Some(serde_json::to_vec(&record).unwrap_or_else(|_| line.to_vec()))
        })
        .in_port("trigger_in")
        .out_port("response_data_out")
        .ctrl_out_port("response_ctrl_out")
}

// ── Main ────────────────────────────────────────────────────────────────

fn main() {
    // Shared ExchangeHeap — diamond branches share SHM pool
    let world =
        Arc::new(VastarRuntimeWorld::new_shared().expect("Failed to init VIL SHM Runtime"));

    // ── Pipeline A: Summary View ────────────────────────────────────────
    let summary_sink = configure_summary_sink();
    let summary_source = configure_summary_source();

    let (_ir_a, (summary_sink_h, summary_source_h)) = vil_workflow! {
        name: "DiamondSummary",
        instances: [ summary_sink, summary_source ],
        routes: [
            summary_sink.trigger_out -> summary_source.trigger_in (LoanWrite),
            summary_source.response_data_out -> summary_sink.response_data_in (LoanWrite),
            summary_source.response_ctrl_out -> summary_sink.response_ctrl_in (Copy),
        ]
    };

    // ── Pipeline B: Detail View ─────────────────────────────────────────
    let detail_sink = configure_detail_sink();
    let detail_source = configure_detail_source();

    let (_ir_b, (detail_sink_h, detail_source_h)) = vil_workflow! {
        name: "DiamondDetail",
        instances: [ detail_sink, detail_source ],
        routes: [
            detail_sink.trigger_out -> detail_source.trigger_in (LoanWrite),
            detail_source.response_data_out -> detail_sink.response_data_in (LoanWrite),
            detail_source.response_ctrl_out -> detail_sink.response_ctrl_in (Copy),
        ]
    };

    // Banner
    println!("╔════════════════════════════════════════════════════════════╗");
    println!("║  104 — Diamond Topology (ShmToken, Multi-Pipeline)       ║");
    println!("╠════════════════════════════════════════════════════════════╣");
    println!("║                                                            ║");
    println!("║           Core Banking Simulator (:18081)                   ║");
    println!("║                  /                  \\                      ║");
    println!("║        Pipeline A              Pipeline B                  ║");
    println!("║      (NPL Summary)         (Full Enrichment)              ║");
    println!("║     :3095/diamond        :3096/diamond-detail             ║");
    println!("║                  \\                  /                      ║");
    println!("║                    Client (gather)                         ║");
    println!("║                                                            ║");
    println!("║  Shared: ExchangeHeap (ShmToken, zero-copy)                ║");
    println!("╚════════════════════════════════════════════════════════════╝");
    println!();
    println!("  Requires: Core Banking Simulator on port 18081");
    println!("    cargo run -p fintec01-simulators");
    println!();
    println!("  Test Summary View (NPL only, compact):");
    println!("  curl -N -X POST http://localhost:{}{} \\", SUMMARY_PORT, SUMMARY_PATH);
    println!("    -H \"Content-Type: application/json\" \\");
    println!("    -d '{{\"request\":\"summary\"}}'");
    println!();
    println!("  Test Detail View (all records, full enrichment):");
    println!("  curl -N -X POST http://localhost:{}{} \\", DETAIL_PORT, DETAIL_PATH);
    println!("    -H \"Content-Type: application/json\" \\");
    println!("    -d '{{\"request\":\"detail\"}}'");
    println!();

    // Build nodes
    let summary_sink_node = HttpSink::from_builder(summary_sink);
    let summary_source_node = HttpSource::from_builder(summary_source);
    let detail_sink_node = HttpSink::from_builder(detail_sink);
    let detail_source_node = HttpSource::from_builder(detail_source);

    // All workers share the SAME world — ShmToken diamond topology
    let t1 = summary_sink_node.run_worker::<ShmToken>(world.clone(), summary_sink_h);
    let t2 = summary_source_node.run_worker::<ShmToken>(world.clone(), summary_source_h);
    let t3 = detail_sink_node.run_worker::<ShmToken>(world.clone(), detail_sink_h);
    let t4 = detail_source_node.run_worker::<ShmToken>(world.clone(), detail_source_h);

    t1.join().expect("SummarySink worker panicked");
    t2.join().expect("SummarySource worker panicked");
    t3.join().expect("DetailSink worker panicked");
    t4.join().expect("DetailSource worker panicked");
}
