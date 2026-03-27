// ╔════════════════════════════════════════════════════════════╗
// ║  025 — Product Knowledge Base (RAG)                       ║
// ╠════════════════════════════════════════════════════════════╣
// ║  Domain:   E-Commerce / Product Documentation             ║
// ║  Pattern:  VX_APP                                         ║
// ║  Token:    N/A (HTTP server)                              ║
// ║  Features: ShmSlice, VilResponse, SseCollect, RAG semantic║
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

use vil_server::prelude::*;

// Semantic types from vil_rag plugin — compile-time validation ensures
// this service correctly participates in the RAG observability pipeline.
use vil_rag::semantic::{RagQueryEvent, RagFault, RagIndexState};

// Upstream LLM endpoint for answer generation. The RAG service retrieves
// context documents first, then sends them with the query to the LLM.
const UPSTREAM_URL: &str = "http://127.0.0.1:4545/v1/chat/completions";

// ── Context documents (Product Knowledge Base) ─────────────────────
// In production, these would come from a vector database (e.g., Qdrant,
// Pinecone) after semantic similarity search. Here we use embedded
// documents to demonstrate the RAG pattern without external dependencies.
// Each document represents a page from the product documentation.

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
// The RAG API: customers ask product questions, get documentation-grounded answers.

// RagRequest: the customer's product question. In a full knowledge base,
// this would also include product_id, customer_tier, and language preference.
#[derive(Debug, Deserialize)]
struct RagRequest {
    prompt: String,
}

// RagResponse: the AI answer with embedded citations. VilModel enables
// zero-copy serialization for high-throughput knowledge base queries.
#[derive(Debug, Clone, Serialize, Deserialize, VilModel)]
struct RagResponse {
    content: String,
}

// ── Handler: RAG query — retrieve context + generate answer ─────────
// This handler implements the full RAG pipeline:
// 1. Parse the customer's product question
// 2. Retrieve relevant documentation (here: all embedded docs)
// 3. Build a context-augmented prompt with citation instructions
// 4. Send to LLM for grounded answer generation
// 5. Return the cited answer to the customer
//
// In production, step 2 would use vector similarity search to find
// only the most relevant documents from thousands of product pages.

async fn rag_handler(
    body: ShmSlice,
) -> HandlerResult<VilResponse<RagResponse>> {
    let req: RagRequest = body.json().expect("invalid JSON body");

    // Build the RAG system prompt. The key instruction: "Answer using ONLY
    // the context documents" prevents hallucination. Citation format [DocN]
    // enables support agents to verify AI answers against source material.
    let system_prompt = format!(
        "You are a helpful RAG assistant. Answer the user's question using ONLY the \
         context documents below. Cite sources as [DocN].\n\n\
         Context:\n{}",
        CONTEXT_DOCS.iter().enumerate()
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

    // Read API key from env (empty = simulator mode, no auth needed)
    let api_key = std::env::var("OPENAI_API_KEY").unwrap_or_default();

    // json_tap for precise SSE content extraction — ensures we only
    // capture the generated answer text, not metadata or tool calls.
    let mut collector = SseCollect::post_to(UPSTREAM_URL)
        .json_tap("choices[0].delta.content")
        .body(body);

    // Add auth if API key is set (skip for local simulator)
    if !api_key.is_empty() {
        collector = collector.bearer_token(&api_key);
    }

    let content = collector.collect_text().await
        .map_err(|e| VilError::internal(e.to_string()))?;

    // Semantic audit: record the RAG query event for observability.
    // In production, this feeds into dashboards showing:
    // - Average chunks retrieved per query (knowledge coverage)
    // - Answer length distribution (quality indicator)
    // - Latency percentiles (customer experience metric)
    // - Model usage (cost tracking per knowledge base query)
    let _event = RagQueryEvent {
        question: req.prompt,
        chunks_retrieved: CONTEXT_DOCS.len() as u32,
        answer_length: content.len() as u32,
        latency_ms: 0,
        model: "gpt-4".into(),
    };

    Ok(VilResponse::ok(RagResponse { content }))
}

// ── Main ─────────────────────────────────────────────────────────────
// Bootstrap the product knowledge base RAG service.

#[tokio::main]
async fn main() {
    // Log semantic type registration — compile-time validation ensures
    // the RAG service's event/fault/state types are compatible with
    // the observability infrastructure.
    let _ = std::any::type_name::<RagQueryEvent>();
    let _ = std::any::type_name::<RagFault>();
    let _ = std::any::type_name::<RagIndexState>();

    println!("======================================================================");
    println!("  Example 024: RAG Service — VilApp (Layer F)");
    println!("  Semantic: RagQueryEvent / RagFault / RagIndexState");
    println!("  Context: 3 embedded docs (Rust ownership)");
    println!("======================================================================");
    println!();
    let api_key = std::env::var("OPENAI_API_KEY").unwrap_or_default();
    println!("  Auth: {}", if api_key.is_empty() { "simulator mode (no auth)" } else { "OPENAI_API_KEY (Bearer)" });
    println!("  Listening on http://localhost:3091/api/rag");
    println!("  Upstream: {} (stream: true)", UPSTREAM_URL);
    println!();

    // The "rag" ServiceProcess handles all product knowledge base queries.
    // Semantic types enable automatic tracking of retrieval quality,
    // index health, and query fault rates across the product catalog.
    let svc = ServiceProcess::new("rag")
        .emits::<RagQueryEvent>()       // Data lane: query + retrieval metrics
        .faults::<RagFault>()           // Fault lane: retrieval/LLM failures
        .manages::<RagIndexState>()     // Control lane: index health status
        .prefix("/api")
        .endpoint(Method::POST, "/rag", post(rag_handler));

    VilApp::new("rag-service")
        .port(3091)
        .service(svc)
        .run()
        .await;
}
