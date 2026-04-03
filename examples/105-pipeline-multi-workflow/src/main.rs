// ╔════════════════════════════════════════════════════════════╗
// ║  105 — Financial Data Hub (Multi-Workflow)                ║
// ╠════════════════════════════════════════════════════════════╣
// ║  Pattern:  MULTI_WORKFLOW                                 ║
// ║  Token:    ShmToken                                       ║
// ║  Nodes:    6 (3 workflows x 2 nodes each)                 ║
// ║  Topology: Three independent workflows in one binary      ║
// ║            Workflow 1: AI Gateway (:3097/ai) -> SSE :4545 ║
// ║            Workflow 2: Credit (:3098/credit) -> NDJSON    ║
// ║            Workflow 3: Inventory (:3099/inventory) -> REST║
// ║  Domain:   Three concurrent pipelines: AI inference +     ║
// ║            credit data + market data — shared ExchangeHeap║
// ╚════════════════════════════════════════════════════════════╝
//
// BUSINESS CONTEXT:
//   Financial data hub that powers a bank's digital platform. Three concurrent
//   pipelines serve different business needs from a single deployment:
//     Workflow 1 (AI Gateway) — LLM-powered financial advisor chatbot
//     Workflow 2 (Credit Ingest) — Core Banking SLIK/NPL credit records for
//       risk assessment, enriched with collectability categories (kolektabilitas
//       1-5) and LTV ratios in real-time
//     Workflow 3 (Inventory) — Product catalog for cross-sell/upsell engine
//   Sharing one ExchangeHeap means the AI advisor can reference credit data
//   without network round-trips — zero-copy cross-workflow data access.
//
// Demonstrates the most advanced multi-workflow pattern: THREE
// independent workflows running concurrently in a single binary,
// all sharing the same ExchangeHeap via ShmToken.
//
// Each workflow handles a different data format:
//   Workflow 1: SSE streaming (AI inference, OpenAI dialect)
//   Workflow 2: NDJSON streaming (Core Banking credit records)
//   Workflow 3: REST single-shot (Product inventory)
//
// This is the canonical example of VIL's process-oriented architecture:
// independent workflows compose freely, share SHM, and run concurrently
// without coordination overhead.
//
// Run:
//   cargo run -p fintec01-simulators   # :18081 (credits), :18092 (products)
//   # Optionally: start an SSE-compatible server on :4545
//   cargo run -p 105-pipeline-multi-workflow
//
// Test:
//   curl -N -X POST http://localhost:3097/ai \
//     -H "Content-Type: application/json" -d '{"prompt":"test"}'
//   curl -N -X POST http://localhost:3098/credit \
//     -H "Content-Type: application/json" -d '{"request":"credits"}'
//   curl -N -X POST http://localhost:3099/inventory \
//     -H "Content-Type: application/json" -d '{"request":"products"}'

use std::sync::Arc;
use vil_sdk::prelude::*;

// ── Semantic Types ──────────────────────────────────────────────────────

/// State for the multi-workflow concurrent system.
#[vil_state]
pub struct MultiWorkflowState {
    pub request_id: u64,
    pub ai_requests: u64,
    pub credit_batches: u64,
    pub inventory_queries: u64,
    pub active_workflows: u8,
}

/// Emitted when any workflow completes a request cycle.
#[vil_event]
pub struct WorkflowCycleCompleted {
    pub workflow_id: u8,
    pub request_id: u64,
    pub latency_ns: u64,
    pub format: u8,
}

/// Faults across all three workflows.
#[vil_fault]
pub enum MultiWorkflowFault {
    AiGatewayTimeout,
    SseParseError,
    CreditSourceTimeout,
    InventorySourceTimeout,
    ShmHeapExhausted,
    WorkflowOrchestrationError,
}

// ── Configuration ───────────────────────────────────────────────────────

// Workflow 1: AI Gateway
const AI_SINK_PORT: u16 = 3097;
const AI_SINK_PATH: &str = "/ai";
const AI_SSE_URL: &str = "http://127.0.0.1:4545/v1/chat/completions";
const AI_JSON_TAP: &str = "choices[0].delta.content";

// Workflow 2: Credit Ingest
const CREDIT_SINK_PORT: u16 = 3098;
const CREDIT_SINK_PATH: &str = "/credit";
const CREDIT_NDJSON_URL: &str = "http://localhost:18081/api/v1/credits/ndjson?count=100";

// Workflow 3: Inventory Check
const INVENTORY_SINK_PORT: u16 = 3099;
const INVENTORY_SINK_PATH: &str = "/inventory";
const INVENTORY_REST_URL: &str = "http://localhost:18092/api/v1/products";

// ── Workflow 1: AI Gateway (SSE) ────────────────────────────────────────

fn configure_ai_sink() -> HttpSinkBuilder {
    HttpSinkBuilder::new("AiGatewaySink")
        .port(AI_SINK_PORT)
        .path(AI_SINK_PATH)
        .out_port("trigger_out")
        .in_port("response_data_in")
        .ctrl_in_port("response_ctrl_in")
}

fn configure_ai_source() -> HttpSourceBuilder {
    let api_key = std::env::var("OPENAI_API_KEY").unwrap_or_default();

    let mut builder = HttpSourceBuilder::new("AiSseSource")
        .url(AI_SSE_URL)
        .format(HttpFormat::SSE)
        .dialect(SseSourceDialect::OpenAi)
        .json_tap(AI_JSON_TAP)
        .post_json(serde_json::json!({
            "model": "gpt-4",
            "messages": [
                { "role": "user", "content": "Multi-workflow concurrent test via VIL" }
            ],
            "stream": true
        }))
        .in_port("trigger_in")
        .out_port("response_data_out")
        .ctrl_out_port("response_ctrl_out");

    if !api_key.is_empty() {
        builder = builder.bearer_token(&api_key);
    }

    builder
}

// ── Workflow 2: Credit Ingest (NDJSON) ──────────────────────────────────

fn configure_credit_sink() -> HttpSinkBuilder {
    HttpSinkBuilder::new("CreditSink")
        .port(CREDIT_SINK_PORT)
        .path(CREDIT_SINK_PATH)
        .out_port("trigger_out")
        .in_port("response_data_in")
        .ctrl_in_port("response_ctrl_in")
}

fn configure_credit_source() -> HttpSourceBuilder {
    HttpSourceBuilder::new("CreditNdjsonSource")
        .url(CREDIT_NDJSON_URL)
        .format(HttpFormat::NDJSON)
        .transform(|line: &[u8]| {
            let mut record: serde_json::Value = serde_json::from_slice(line).ok()?;
            // Enrich with workflow tag and risk category.
            // Indonesian banking regulation (OJK/BI) classifies credit quality
            // into 5 levels — this transform adds the human-readable category.
            record["_workflow"] = serde_json::json!("CREDIT_INGEST");
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
            record["_ltv_ratio"] = serde_json::json!(((saldo / plafon * 100.0).round() / 100.0));
            Some(serde_json::to_vec(&record).unwrap_or_else(|_| line.to_vec()))
        })
        .in_port("trigger_in")
        .out_port("response_data_out")
        .ctrl_out_port("response_ctrl_out")
}

// ── Workflow 3: Inventory Check (REST single-shot) ──────────────────────

fn configure_inventory_sink() -> HttpSinkBuilder {
    HttpSinkBuilder::new("InventorySink")
        .port(INVENTORY_SINK_PORT)
        .path(INVENTORY_SINK_PATH)
        .out_port("trigger_out")
        .in_port("response_data_in")
        .ctrl_in_port("response_ctrl_in")
}

fn configure_inventory_source() -> HttpSourceBuilder {
    HttpSourceBuilder::new("InventoryRestSource")
        .url(INVENTORY_REST_URL)
        .format(HttpFormat::Raw)
        .transform(|body: &[u8]| {
            // REST single-shot: tag with workflow identifier
            let mut record: serde_json::Value = serde_json::from_slice(body).ok()?;
            if let Some(obj) = record.as_object_mut() {
                obj.insert(
                    "_workflow".to_string(),
                    serde_json::json!("INVENTORY_CHECK"),
                );
                obj.insert("_format".to_string(), serde_json::json!("REST_SINGLE_SHOT"));
            }
            Some(serde_json::to_vec(&record).unwrap_or_else(|_| body.to_vec()))
        })
        .in_port("trigger_in")
        .out_port("response_data_out")
        .ctrl_out_port("response_ctrl_out")
}

// ── Main ────────────────────────────────────────────────────────────────

fn main() {
    // Single shared ExchangeHeap for ALL three workflows.
    // Business advantage: the AI advisor workflow can read credit data enriched
    // by the credit ingest workflow without serialization — zero-copy IPC.
    let world = Arc::new(VastarRuntimeWorld::new_shared().expect("Failed to init VIL SHM Runtime"));

    // ── Workflow 1: AI Gateway (SSE) ────────────────────────────────────
    let ai_sink = configure_ai_sink();
    let ai_source = configure_ai_source();

    let (_ir_1, (ai_sink_h, ai_source_h)) = vil_workflow! {
        name: "AiGatewayWorkflow",
        instances: [ ai_sink, ai_source ],
        routes: [
            ai_sink.trigger_out -> ai_source.trigger_in (LoanWrite),
            ai_source.response_data_out -> ai_sink.response_data_in (LoanWrite),
            ai_source.response_ctrl_out -> ai_sink.response_ctrl_in (Copy),
        ]
    };

    // ── Workflow 2: Credit Ingest (NDJSON) ──────────────────────────────
    let credit_sink = configure_credit_sink();
    let credit_source = configure_credit_source();

    let (_ir_2, (credit_sink_h, credit_source_h)) = vil_workflow! {
        name: "CreditIngestWorkflow",
        instances: [ credit_sink, credit_source ],
        routes: [
            credit_sink.trigger_out -> credit_source.trigger_in (LoanWrite),
            credit_source.response_data_out -> credit_sink.response_data_in (LoanWrite),
            credit_source.response_ctrl_out -> credit_sink.response_ctrl_in (Copy),
        ]
    };

    // ── Workflow 3: Inventory Check (REST) ──────────────────────────────
    let inventory_sink = configure_inventory_sink();
    let inventory_source = configure_inventory_source();

    let (_ir_3, (inventory_sink_h, inventory_source_h)) = vil_workflow! {
        name: "InventoryCheckWorkflow",
        instances: [ inventory_sink, inventory_source ],
        routes: [
            inventory_sink.trigger_out -> inventory_source.trigger_in (LoanWrite),
            inventory_source.response_data_out -> inventory_sink.response_data_in (LoanWrite),
            inventory_source.response_ctrl_out -> inventory_sink.response_ctrl_in (Copy),
        ]
    };

    // Banner
    println!("╔════════════════════════════════════════════════════════════╗");
    println!("║  105 — Multi-Workflow Concurrent (ShmToken)              ║");
    println!("╠════════════════════════════════════════════════════════════╣");
    println!("║                                                            ║");
    println!("║  Workflow 1 — AI Gateway (SSE):                            ║");
    println!("║    Sink(:3097/ai) ──> Source(SSE :4545, OpenAI dialect)  ║");
    println!("║                                                            ║");
    println!("║  Workflow 2 — Credit Ingest (NDJSON):                      ║");
    println!("║    Sink(:3098/credit) ──> Source(NDJSON :18081)          ║");
    println!("║                                                            ║");
    println!("║  Workflow 3 — Inventory Check (REST):                      ║");
    println!("║    Sink(:3099/inventory) ──> Source(REST :18092)         ║");
    println!("║                                                            ║");
    println!("║  Shared: ExchangeHeap (ShmToken, zero-copy)                ║");
    println!("║  Formats: SSE + NDJSON + REST (mixed protocols)            ║");
    println!("╚════════════════════════════════════════════════════════════╝");
    println!();
    let api_key = std::env::var("OPENAI_API_KEY").unwrap_or_default();
    println!(
        "  AI Auth: {}",
        if api_key.is_empty() {
            "simulator mode (no auth)"
        } else {
            "OPENAI_API_KEY (Bearer)"
        }
    );
    println!();
    println!("  Requires:");
    println!("    - Core Banking Simulator on port 18081 (credits NDJSON)");
    println!("    - Product Inventory on port 18092 (REST)");
    println!("    - SSE server on port 4545 (AI inference, optional)");
    println!("    cargo run -p fintec01-simulators");
    println!();
    println!("  Test AI Gateway:");
    println!(
        "  curl -N -X POST http://localhost:{}{} \\",
        AI_SINK_PORT, AI_SINK_PATH
    );
    println!("    -H \"Content-Type: application/json\" \\");
    println!("    -d '{{\"prompt\":\"test\"}}'");
    println!();
    println!("  Test Credit Ingest:");
    println!(
        "  curl -N -X POST http://localhost:{}{} \\",
        CREDIT_SINK_PORT, CREDIT_SINK_PATH
    );
    println!("    -H \"Content-Type: application/json\" \\");
    println!("    -d '{{\"request\":\"credits\"}}'");
    println!();
    println!("  Test Inventory Check:");
    println!(
        "  curl -N -X POST http://localhost:{}{} \\",
        INVENTORY_SINK_PORT, INVENTORY_SINK_PATH
    );
    println!("    -H \"Content-Type: application/json\" \\");
    println!("    -d '{{\"request\":\"products\"}}'");
    println!();

    // Build all nodes
    let ai_sink_node = HttpSink::from_builder(ai_sink);
    let ai_source_node = HttpSource::from_builder(ai_source);
    let credit_sink_node = HttpSink::from_builder(credit_sink);
    let credit_source_node = HttpSource::from_builder(credit_source);
    let inventory_sink_node = HttpSink::from_builder(inventory_sink);
    let inventory_source_node = HttpSource::from_builder(inventory_source);

    // All 6 workers share the SAME world — ShmToken multi-workflow concurrency.
    // Each worker is a separate OS thread for true parallelism on multi-core CPUs.
    let t1 = ai_sink_node.run_worker::<ShmToken>(world.clone(), ai_sink_h);
    let t2 = ai_source_node.run_worker::<ShmToken>(world.clone(), ai_source_h);
    let t3 = credit_sink_node.run_worker::<ShmToken>(world.clone(), credit_sink_h);
    let t4 = credit_source_node.run_worker::<ShmToken>(world.clone(), credit_source_h);
    let t5 = inventory_sink_node.run_worker::<ShmToken>(world.clone(), inventory_sink_h);
    let t6 = inventory_source_node.run_worker::<ShmToken>(world.clone(), inventory_source_h);

    t1.join().expect("AiGatewaySink worker panicked");
    t2.join().expect("AiSseSource worker panicked");
    t3.join().expect("CreditSink worker panicked");
    t4.join().expect("CreditNdjsonSource worker panicked");
    t5.join().expect("InventorySink worker panicked");
    t6.join().expect("InventoryRestSource worker panicked");
}
