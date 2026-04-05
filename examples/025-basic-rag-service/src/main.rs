// ╔════════════════════════════════════════════════════════════╗
// ║  025 — Product Knowledge Base (RAG)                       ║
// ╠════════════════════════════════════════════════════════════╣
// ║  Domain:   E-Commerce / Product Documentation             ║
// ║  Pattern:  VX_APP                                         ║
// ║  Token:    N/A (HTTP server)                              ║
// ║  Features: ShmSlice, VilResponse, SseCollect, RAG semantic║
// ║            RagQueryEvent, RagFault, RagIndexState          ║
// ╠════════════════════════════════════════════════════════════╣
// ║  Business: RAG-powered product documentation search.       ║
// ║  Every query produces RagQueryEvent (Data Lane) for        ║
// ║  retrieval quality audit. RagIndexState tracks index       ║
// ║  health for operations dashboards.                         ║
// ╚════════════════════════════════════════════════════════════╝
//
// Business Context:
//   Search product documentation and manuals to answer customer questions.
//   In e-commerce and SaaS platforms, RAG (Retrieval-Augmented Generation)
//   dramatically improves support quality by grounding AI answers in
//   actual product documentation:
//
//   - Reduces hallucination: answers cite real docs, not made-up facts
//   - Always current: retrieval pulls from the latest product manuals
//   - Traceable: each answer includes [DocN] citations for verification
//   - Scalable: handles thousands of product SKUs without fine-tuning
//
// RAG Flow:
//   1. Customer asks a question about a product
//   2. Retriever searches the product knowledge base (here: embedded docs)
//   3. Relevant documents are injected into the LLM's context window
//   4. LLM generates an answer grounded in the retrieved documents
//   5. Citations are included so support agents can verify accuracy
//
// Run:
//   cargo run -p basic-usage-rag-service
//
// Test:
//   curl -X POST -H "Content-Type: application/json" \
//     -d '{"prompt": "What is Rust ownership?"}' \
//     http://localhost:3091/api/rag
//   curl http://localhost:3091/api/usage

use std::sync::{Arc, Mutex};
use std::time::Instant;

use vil_server::prelude::*;

// Semantic types from vil_rag plugin — compile-time validation ensures
// this service correctly participates in the RAG observability pipeline.
use vil_rag::semantic::{RagFault, RagIndexState, RagQueryEvent};

// Upstream LLM endpoint for answer generation. The RAG service retrieves
// context documents first, then sends them with the query to the LLM.
const UPSTREAM_URL: &str = "http://127.0.0.1:4545/v1/chat/completions";

// ── Context documents (Product Knowledge Base) ─────────────────────
// In production, these would come from a vector database (e.g., Qdrant,
// Pinecone) after semantic similarity search. Here we use embedded
// documents to demonstrate the RAG pattern without external dependencies.

const CONTEXT_DOCS: &[&str] = &[
    "[Doc1] Rust is a systems programming language focused on safety, speed, and \
     concurrency. It achieves memory safety without a garbage collector.",
    "[Doc2] The Rust ownership model has three rules: each value has exactly one owner, \
     when the owner goes out of scope the value is dropped, and ownership can be \
     transferred via move semantics or borrowed via references.",
    "[Doc3] Rust's borrow checker enforces that references must always be valid, \
     and you can have either one mutable reference or any number of immutable references.",
];

// ── Request / Response ───────────────────────────────────────────────

#[derive(Debug, Deserialize)]
struct RagRequest {
    prompt: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, VilModel)]
struct RagResponse {
    content: String,
    chunks_used: u32,
    latency_ms: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize, VilModel)]
struct UsageResponse {
    total_queries: u64,
    total_chunks_retrieved: u64,
    avg_latency_ms: f64,
    index_doc_count: u64,
    index_chunk_count: u64,
}

// ── Shared state ────────────────────────────────────────────────────

struct AppState {
    index: Mutex<RagIndexState>,
    total_queries: Mutex<u64>,
    total_chunks_retrieved: Mutex<u64>,
    latency_sum_ms: Mutex<f64>,
}

// ── Handler: RAG query — retrieve context + generate answer ─────────

async fn rag_handler(
    ctx: ServiceCtx,
    body: ShmSlice,
) -> HandlerResult<VilResponse<RagResponse>> {
    let req: RagRequest = body.json()
        .map_err(|_| VilError::bad_request("invalid JSON"))?;

    if req.prompt.trim().is_empty() {
        return Err(VilError::bad_request("prompt is required"));
    }

    let start = Instant::now();

    // Build the RAG system prompt with context documents.
    let system_prompt = format!(
        "You are a helpful RAG assistant. Answer the user's question using ONLY the \
         context documents below. Cite sources as [DocN].\n\n\
         Context:\n{}",
        CONTEXT_DOCS
            .iter()
            .enumerate()
            .map(|(i, d)| format!("[Doc{}] {}", i + 1, d))
            .collect::<Vec<_>>()
            .join("\n\n")
    );

    let body = serde_json::json!({
        "model": "gpt-4",
        "messages": [
            {"role": "system", "content": system_prompt},
            {"role": "user", "content": req.prompt}
        ],
        "stream": true
    });

    let api_key = std::env::var("OPENAI_API_KEY").unwrap_or_default();

    let mut collector = SseCollect::post_to(UPSTREAM_URL)
        .json_tap("choices[0].delta.content")
        .body(body);

    if !api_key.is_empty() {
        collector = collector.bearer_token(&api_key);
    }

    let content = collector
        .collect_text()
        .await
        .map_err(|e| VilError::internal(e.to_string()))?;

    let latency_ms = start.elapsed().as_millis() as u64;
    let chunks_retrieved = CONTEXT_DOCS.len() as u32;

    // ── Construct RagQueryEvent (from vil_rag::semantic) ──
    // Semantic audit record for retrieval quality monitoring.
    let event = RagQueryEvent {
        question: req.prompt.clone(),
        chunks_retrieved,
        answer_length: content.len() as u32,
        latency_ms,
        model: "gpt-4".into(),
    };

    // ── Update query tracking state ──
    let state = ctx.state::<Arc<AppState>>()
        .map_err(|_| VilError::internal("state not found"))?;
    {
        *state.total_queries.lock().unwrap() += 1;
        *state.total_chunks_retrieved.lock().unwrap() += event.chunks_retrieved as u64;
        *state.latency_sum_ms.lock().unwrap() += event.latency_ms as f64;
    }

    // Log RAG query for observability
    eprintln!(
        "[RAG] query={} chunks={} answer_len={} latency_ms={} model=gpt-4",
        req.prompt.chars().take(50).collect::<String>(),
        chunks_retrieved,
        content.len(),
        latency_ms,
    );

    Ok(VilResponse::ok(RagResponse {
        content,
        chunks_used: chunks_retrieved,
        latency_ms,
    }))
}

// ── Handler: usage stats ─────────────────────────────────────────────
// Exposes RagIndexState and query metrics for monitoring dashboards.

async fn usage_handler(ctx: ServiceCtx) -> HandlerResult<VilResponse<UsageResponse>> {
    let state = ctx.state::<Arc<AppState>>()
        .map_err(|_| VilError::internal("state not found"))?;

    let total_queries = *state.total_queries.lock().unwrap();
    let total_chunks = *state.total_chunks_retrieved.lock().unwrap();
    let latency_sum = *state.latency_sum_ms.lock().unwrap();
    let index = state.index.lock().unwrap();

    let avg_latency_ms = if total_queries > 0 {
        latency_sum / total_queries as f64
    } else {
        0.0
    };

    Ok(VilResponse::ok(UsageResponse {
        total_queries,
        total_chunks_retrieved: total_chunks,
        avg_latency_ms,
        index_doc_count: index.doc_count,
        index_chunk_count: index.chunk_count,
    }))
}

// ── Main ─────────────────────────────────────────────────────────────

#[tokio::main]
async fn main() {
    println!("╔══════════════════════════════════════════════════════════════╗");
    println!("║  025 — RAG Service (VilApp + SseCollect + RAG Semantic)    ║");
    println!("║  Events: RagQueryEvent | Faults: RagFault                   ║");
    println!("║  State: RagIndexState (index health tracking)               ║");
    println!("╚══════════════════════════════════════════════════════════════╝");
    println!();
    let api_key = std::env::var("OPENAI_API_KEY").unwrap_or_default();
    println!(
        "  Auth: {}",
        if api_key.is_empty() {
            "simulator mode (no auth)"
        } else {
            "OPENAI_API_KEY (Bearer)"
        }
    );
    println!("  RAG:   http://localhost:3091/api/rag");
    println!("  Usage: http://localhost:3091/api/usage");
    println!("  Upstream: {} (stream: true)", UPSTREAM_URL);
    println!();

    let app_state = Arc::new(AppState {
        index: Mutex::new(RagIndexState {
            doc_count: CONTEXT_DOCS.len() as u64,
            chunk_count: CONTEXT_DOCS.len() as u64,
            store_type: "embedded".into(),
            ..Default::default()
        }),
        total_queries: Mutex::new(0),
        total_chunks_retrieved: Mutex::new(0),
        latency_sum_ms: Mutex::new(0.0),
    });

    // The "rag" ServiceProcess handles all product knowledge base queries.
    let svc = ServiceProcess::new("rag")
        .emits::<RagQueryEvent>()
        .faults::<RagFault>()
        .manages::<RagIndexState>()
        .prefix("/api")
        .endpoint(Method::POST, "/rag", post(rag_handler))
        .endpoint(Method::GET, "/usage", get(usage_handler))
        .state(app_state);

    VilApp::new("rag-service")
        .port(3091)
        .observer(true)
        .service(svc)
        .run()
        .await;
}
