// ╔════════════════════════════════════════════════════════════╗
// ║  103 — Multi-Source Financial Data Aggregator (Fan-In)    ║
// ╠════════════════════════════════════════════════════════════╣
// ║  Domain:   Banking — Cross-System Data Aggregation         ║
// ║  Pattern:  MULTI_PIPELINE (Fan-In Gather)                 ║
// ║  Token:    ShmToken (shared ExchangeHeap)                 ║
// ║  Nodes:    4 (2 pipelines x 2 nodes each)                 ║
// ╠════════════════════════════════════════════════════════════╣
// ║  Business: Aggregates data from multiple financial         ║
// ║  systems into a unified view:                              ║
// ║    Pipeline A: Credit records (Core Banking, NDJSON)      ║
// ║    Pipeline B: Product inventory (REST service)            ║
// ║  Client triggers each pipeline independently and gathers  ║
// ║  results for consolidated reporting (e.g., customer 360). ║
// ║  Both pipelines share SHM for zero-copy transport.        ║
// ╚════════════════════════════════════════════════════════════╝
//
// Demonstrates the Fan-In Gather pattern: two independent pipelines
// each consume a DIFFERENT upstream data source, but both share the
// same ExchangeHeap (ShmToken). The client triggers each endpoint
// independently and gathers results from both:
//
//   Pipeline A: Credit records from Core Banking Simulator (NDJSON)
//   Pipeline B: Product inventory from REST Inventory Service
//
// Both pipelines run concurrently in the same binary, sharing SHM.
// This is the inverse of fan-out: multiple sources converge at
// the client level (fan-in gather).
//
// Run:
//   cargo run -p fintec01-simulators   # provides both :18081 and :18092
//   cargo run -p 103-pipeline-fanin-gather
//
// Test:
//   # Gather credit data
//   curl -N -X POST http://localhost:3093/gather \
//     -H "Content-Type: application/json" -d '{"request":"credits"}'
//
//   # Gather inventory data
//   curl -N -X POST http://localhost:3094/inventory \
//     -H "Content-Type: application/json" -d '{"request":"inventory"}'

use std::sync::Arc;
use vil_sdk::prelude::*;

// ── Semantic Types ──────────────────────────────────────────────────────

/// State for the fan-in gather across heterogeneous data sources.
#[vil_state]
pub struct GatherState {
    pub request_id: u64,
    pub credit_records_received: u64,
    pub inventory_items_received: u64,
    pub gather_complete: bool,
}

/// Emitted when a gather operation completes from either source.
#[vil_event]
pub struct GatherCompleted {
    pub source_name: u8,
    pub record_count: u64,
    pub latency_ns: u64,
    pub format: u8,
}

/// Faults for the fan-in gather topology.
#[vil_fault]
pub enum GatherFault {
    CreditSourceTimeout,
    InventorySourceTimeout,
    FormatMismatch,
    ShmHeapExhausted,
    GatherIncomplete,
}

// ── Configuration ───────────────────────────────────────────────────────

const CREDIT_SINK_PORT: u16 = 3093;
const CREDIT_SINK_PATH: &str = "/gather";
const INVENTORY_SINK_PORT: u16 = 3094;
const INVENTORY_SINK_PATH: &str = "/inventory";

const CREDIT_NDJSON_URL: &str = "http://localhost:18081/api/v1/credits/ndjson?count=100";
const INVENTORY_REST_URL: &str = "http://localhost:18092/api/v1/products";

// ── Pipeline A: Credit Records (NDJSON) ─────────────────────────────────

fn configure_credit_sink() -> HttpSinkBuilder {
    HttpSinkBuilder::new("CreditGatherSink")
        .port(CREDIT_SINK_PORT)
        .path(CREDIT_SINK_PATH)
        .out_port("trigger_out")
        .in_port("response_data_in")
        .ctrl_in_port("response_ctrl_in")
}

fn configure_credit_source() -> HttpSourceBuilder {
    HttpSourceBuilder::new("CreditSource")
        .url(CREDIT_NDJSON_URL)
        .format(HttpFormat::NDJSON)
        .transform(|line: &[u8]| {
            let mut record: serde_json::Value = serde_json::from_slice(line).ok()?;
            // Tag with source origin for fan-in identification
            record["_source"] = serde_json::json!("CORE_BANKING");
            record["_format"] = serde_json::json!("NDJSON");
            // Compute delinquency flag
            let kol = record["kolektabilitas"].as_u64().unwrap_or(0);
            record["_is_delinquent"] = serde_json::json!(kol >= 3);
            Some(serde_json::to_vec(&record).unwrap_or_else(|_| line.to_vec()))
        })
        .in_port("trigger_in")
        .out_port("response_data_out")
        .ctrl_out_port("response_ctrl_out")
}

// ── Pipeline B: Inventory (REST single-shot) ────────────────────────────

fn configure_inventory_sink() -> HttpSinkBuilder {
    HttpSinkBuilder::new("InventoryGatherSink")
        .port(INVENTORY_SINK_PORT)
        .path(INVENTORY_SINK_PATH)
        .out_port("trigger_out")
        .in_port("response_data_in")
        .ctrl_in_port("response_ctrl_in")
}

fn configure_inventory_source() -> HttpSourceBuilder {
    HttpSourceBuilder::new("InventorySource")
        .url(INVENTORY_REST_URL)
        .format(HttpFormat::Raw)
        .transform(|body: &[u8]| {
            // REST returns a single JSON array or object — tag with source
            let mut record: serde_json::Value = serde_json::from_slice(body).ok()?;
            if let Some(obj) = record.as_object_mut() {
                obj.insert(
                    "_source".to_string(),
                    serde_json::json!("INVENTORY_SERVICE"),
                );
                obj.insert("_format".to_string(), serde_json::json!("REST"));
            }
            Some(serde_json::to_vec(&record).unwrap_or_else(|_| body.to_vec()))
        })
        .in_port("trigger_in")
        .out_port("response_data_out")
        .ctrl_out_port("response_ctrl_out")
}

// ── Main ────────────────────────────────────────────────────────────────

fn main() {
    // Shared ExchangeHeap — both pipelines on same SHM pool
    let world = Arc::new(VastarRuntimeWorld::new_shared().expect("Failed to init VIL SHM Runtime"));

    // ── Pipeline A: Credit NDJSON ───────────────────────────────────────
    let credit_sink = configure_credit_sink();
    let credit_source = configure_credit_source();

    let (_ir_a, (credit_sink_h, credit_source_h)) = vil_workflow! {
        name: "CreditGatherPipeline",
        instances: [ credit_sink, credit_source ],
        routes: [
            credit_sink.trigger_out -> credit_source.trigger_in (LoanWrite),
            credit_source.response_data_out -> credit_sink.response_data_in (LoanWrite),
            credit_source.response_ctrl_out -> credit_sink.response_ctrl_in (Copy),
        ]
    };

    // ── Pipeline B: Inventory REST ──────────────────────────────────────
    let inventory_sink = configure_inventory_sink();
    let inventory_source = configure_inventory_source();

    let (_ir_b, (inventory_sink_h, inventory_source_h)) = vil_workflow! {
        name: "InventoryGatherPipeline",
        instances: [ inventory_sink, inventory_source ],
        routes: [
            inventory_sink.trigger_out -> inventory_source.trigger_in (LoanWrite),
            inventory_source.response_data_out -> inventory_sink.response_data_in (LoanWrite),
            inventory_source.response_ctrl_out -> inventory_sink.response_ctrl_in (Copy),
        ]
    };

    // Banner
    println!("╔════════════════════════════════════════════════════════════╗");
    println!("║  103 — Fan-In Gather (ShmToken, Multi-Pipeline)          ║");
    println!("╠════════════════════════════════════════════════════════════╣");
    println!("║                                                            ║");
    println!("║  Pipeline A (Credit Records):                              ║");
    println!("║    Sink(:3093/gather) ──> Source(NDJSON :18081)           ║");
    println!("║                                                            ║");
    println!("║  Pipeline B (Inventory):                                   ║");
    println!("║    Sink(:3094/inventory) ──> Source(REST :18092)          ║");
    println!("║                                                            ║");
    println!("║  Pattern: Fan-In — multiple sources, client gathers       ║");
    println!("║  Shared:  ExchangeHeap (ShmToken, zero-copy)              ║");
    println!("╚════════════════════════════════════════════════════════════╝");
    println!();
    println!("  Requires: Simulators on ports 18081 (credits) and 18092 (products)");
    println!("    cargo run -p fintec01-simulators");
    println!();
    println!("  Test Credit Gather:");
    println!(
        "  curl -N -X POST http://localhost:{}{} \\",
        CREDIT_SINK_PORT, CREDIT_SINK_PATH
    );
    println!("    -H \"Content-Type: application/json\" \\");
    println!("    -d '{{\"request\":\"credits\"}}'");
    println!();
    println!("  Test Inventory Gather:");
    println!(
        "  curl -N -X POST http://localhost:{}{} \\",
        INVENTORY_SINK_PORT, INVENTORY_SINK_PATH
    );
    println!("    -H \"Content-Type: application/json\" \\");
    println!("    -d '{{\"request\":\"inventory\"}}'");
    println!();

    // Build nodes
    let credit_sink_node = HttpSink::from_builder(credit_sink);
    let credit_source_node = HttpSource::from_builder(credit_source);
    let inventory_sink_node = HttpSink::from_builder(inventory_sink);
    let inventory_source_node = HttpSource::from_builder(inventory_source);

    // All workers share the SAME world — ShmToken fan-in from multiple sources
    let t1 = credit_sink_node.run_worker::<ShmToken>(world.clone(), credit_sink_h);
    let t2 = credit_source_node.run_worker::<ShmToken>(world.clone(), credit_source_h);
    let t3 = inventory_sink_node.run_worker::<ShmToken>(world.clone(), inventory_sink_h);
    let t4 = inventory_source_node.run_worker::<ShmToken>(world.clone(), inventory_source_h);

    t1.join().expect("CreditGatherSink worker panicked");
    t2.join().expect("CreditSource worker panicked");
    t3.join().expect("InventoryGatherSink worker panicked");
    t4.join().expect("InventorySource worker panicked");
}
