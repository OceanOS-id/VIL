// ╔════════════════════════════════════════════════════════════╗
// ║  301 — Internal Wiki Search Engine                        ║
// ╠════════════════════════════════════════════════════════════╣
// ║  Domain:   Enterprise — Knowledge Management               ║
// ║  Pattern:  VX_APP                                         ║
// ║  Token:    N/A (HTTP server)                              ║
// ║  Macros:   ShmSlice, ServiceCtx, VilResponse, #[vil_fault]║
// ╠════════════════════════════════════════════════════════════╣
// ║  Business: Semantic search over internal wiki/docs.        ║
// ║  Flow: embed query -> cosine similarity -> top-k docs ->  ║
// ║  augment LLM prompt with context -> grounded answer.       ║
// ║  Reduces time-to-answer for engineering questions from     ║
// ║  minutes (manual search) to seconds (RAG-powered).        ║
// ╚════════════════════════════════════════════════════════════╝
//
// Run:
//   cargo run -p rag-plugin-usage-basic-query
//
// Test:
//   curl -X POST -H "Content-Type: application/json" \
//     -d '{"prompt": "What is Rust ownership?"}' \
//     http://localhost:3110/api/rag

use vil_server::prelude::*;

// Semantic types from vil_rag plugin
use vil_rag::semantic::{RagQueryEvent, RagIngestEvent, RagFault, RagIndexState};

const UPSTREAM_URL: &str = "http://127.0.0.1:4545/v1/chat/completions";

// ── Semantic Types ────────────────────────────────────────────────────

#[derive(Clone, Debug)]
/// Wiki search state — tracks query volume and search quality metrics.
pub struct VectorSearchState {
    pub total_queries: u64,
    pub total_docs_searched: u64,
    pub avg_similarity_score: f64,
}

#[derive(Clone, Debug)]
/// Search event — logged for search quality monitoring and relevance tuning.
pub struct VectorSearchEvent {
    pub query: String,
    pub top_k: u32,
    pub best_score: f64,
    pub docs_matched: u32,
}

#[vil_fault]
/// Search faults — triggers fallback behavior or admin notification.
pub enum VectorSearchFault {
    NoDocsMatched,
    SimilarityBelowThreshold,
    LlmUpstreamError,
}

// ── Context documents — Internal wiki knowledge base ─────────────────
// In production, these documents would be stored in a vector database
// (e.g., Qdrant, Pinecone, pgvector) with real embeddings from a model
// like text-embedding-ada-002. Here we use mock 4-dim embeddings for demo.
// Each document represents a wiki article that employees might search for.

/// A document in the internal wiki knowledge base
struct Document {
    id: &'static str,
    title: &'static str,
    content: &'static str,
    // Mock embedding: 4-dim vector for demo purposes
    embedding: [f64; 4],
}

const DOCS: &[Document] = &[
    Document {
        id: "Doc1",
        title: "Rust Ownership",
        content: "Rust is a systems programming language focused on safety, speed, and \
                  concurrency. It achieves memory safety without a garbage collector through its \
                  ownership system. Each value has exactly one owner, and when the owner goes \
                  out of scope the value is dropped.",
        embedding: [0.9, 0.3, 0.1, 0.2],
    },
    Document {
        id: "Doc2",
        title: "Borrowing and References",
        content: "Rust's ownership model includes borrowing: you can have either one mutable \
                  reference or any number of immutable references at the same time. This \
                  prevents data races at compile time. References are always valid — no null \
                  or dangling pointers.",
        embedding: [0.8, 0.4, 0.1, 0.3],
    },
    Document {
        id: "Doc3",
        title: "Traits and Generics",
        content: "Rust's trait system enables polymorphism. A trait defines shared behavior \
                  that types can implement. Generic functions use trait bounds to constrain \
                  which types they accept, enabling zero-cost abstractions at compile time.",
        embedding: [0.2, 0.8, 0.6, 0.1],
    },
];

/// Mock embedding: converts query to a 4-dim vector based on keyword presence.
/// In production, use a real embedding model (OpenAI, Cohere, local ONNX).
/// The 4 dimensions roughly correspond to: ownership, traits, types, borrowing.
fn mock_embed_query(query: &str) -> [f64; 4] {
    let q = query.to_lowercase();
    [
        if q.contains("ownership") || q.contains("memory") || q.contains("safety") { 0.9 } else { 0.1 },
        if q.contains("trait") || q.contains("generic") || q.contains("polymorphism") { 0.9 } else { 0.1 },
        if q.contains("type") || q.contains("abstract") { 0.7 } else { 0.1 },
        if q.contains("borrow") || q.contains("reference") || q.contains("lifetime") { 0.9 } else { 0.1 },
    ]
}

/// Cosine similarity between two vectors
/// Cosine similarity between query and document embeddings.
/// In production, this runs on GPU-accelerated vector DB (Qdrant, pgvector).
fn cosine_similarity(a: &[f64; 4], b: &[f64; 4]) -> f64 {
    let dot: f64 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let mag_a: f64 = a.iter().map(|x| x * x).sum::<f64>().sqrt();
    let mag_b: f64 = b.iter().map(|x| x * x).sum::<f64>().sqrt();
    if mag_a == 0.0 || mag_b == 0.0 { 0.0 } else { dot / (mag_a * mag_b) }
}

// ── Request / Response ───────────────────────────────────────────────
// RagRequest: employee submits a natural language question.
// RagResponse: grounded answer with docs_used (which wiki articles were cited)
// and similarity_scores (how relevant each document was to the query).

/// Wiki search request — natural language question from an employee
#[derive(Debug, Deserialize)]
struct RagRequest {
    prompt: String,   // e.g., "What is Rust ownership?" or "How to configure VPN?"
}

/// Wiki search response — LLM answer grounded in specific wiki articles
#[derive(Debug, Clone, Serialize, Deserialize, VilModel)]
struct RagResponse {
    content: String,                 // Generated answer citing [DocN] sources
    docs_used: Vec<String>,          // Wiki article IDs used as context
    similarity_scores: Vec<f64>,     // Cosine similarity scores for transparency
}

// ── Handler ──────────────────────────────────────────────────────────
// RAG pipeline: embed query -> retrieve top-k docs -> augment LLM prompt ->
// generate grounded answer with [DocN] citations. This flow eliminates
// hallucination by constraining the LLM to only use provided context.

/// POST /api/rag — semantic search with LLM-generated answer
async fn rag_handler(
    ctx: ServiceCtx, body: ShmSlice,
) -> HandlerResult<VilResponse<RagResponse>> {
    let req: RagRequest = body.json().expect("invalid JSON body");
    // Step 1: Embed query and compute similarities
    let query_emb = mock_embed_query(&req.prompt);
    // Step 1: Compute similarity scores between query and all documents in the wiki index
    let mut scored: Vec<(usize, f64)> = DOCS.iter().enumerate()
        .map(|(i, doc)| (i, cosine_similarity(&query_emb, &doc.embedding)))
        .collect();
    scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

    // Step 2: Take top-2 documents
    let top_k = 2;
    // Step 2: Select top-k most relevant documents for LLM context augmentation
    let top_docs: Vec<&Document> = scored.iter().take(top_k).map(|(i, _)| &DOCS[*i]).collect();
    let scores: Vec<f64> = scored.iter().take(top_k).map(|(_, s)| (*s * 100.0).round() / 100.0).collect();
    let doc_ids: Vec<String> = top_docs.iter().map(|d| d.id.to_string()).collect();

    // Step 3: Build context for LLM
    let context: String = top_docs.iter()
        .map(|d| format!("[{}] {}: {}", d.id, d.title, d.content))
        .collect::<Vec<_>>()
        .join("\n\n");

    // Step 3: Build the RAG prompt — system instructions + retrieved document context
    let system_prompt = format!(
        "You are a helpful RAG assistant. Answer the user's question using ONLY the \
         context documents below. Cite sources as [DocN].\n\nContext:\n{}",
        context
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

    // Step 4: Collect the full SSE response into a single text string for the client
    let content = collector.collect_text().await
        .map_err(|e| VilError::internal(e.to_string()))?;

    // Semantic audit
    let _event = RagQueryEvent {
        question: req.prompt,
        chunks_retrieved: top_k as u32,
        answer_length: content.len() as u32,
        latency_ms: 0,
        model: "gpt-4".into(),
    };

    // Return the grounded answer with provenance (which docs were used and their relevance)
    Ok(VilResponse::ok(RagResponse {
        content,
        docs_used: doc_ids,
        similarity_scores: scores,
    }))
}

// ── Main ─────────────────────────────────────────────────────────────

#[tokio::main]
// ── Main — Internal Wiki Search Engine ─────────────────────────────
// Assembles the RAG search service with semantic type registration
// for observability. Documents are pre-indexed with mock embeddings.
async fn main() {
    let _ = std::any::type_name::<RagQueryEvent>();
    let _ = std::any::type_name::<RagIngestEvent>();
    let _ = std::any::type_name::<RagFault>();
    let _ = std::any::type_name::<RagIndexState>();

    println!("╔══════════════════════════════════════════════════════════════╗");
    println!("║  301 — RAG Basic Vector Search (VilApp)                    ║");
    // Banner: display pipeline topology and connection info
    println!("║  Pattern: VX_APP | Token: N/A                              ║");
    println!("║  Unique: Static docs + cosine similarity + top-k retrieval ║");
    println!("╚══════════════════════════════════════════════════════════════╝");
    println!();
    // Display the number of indexed documents in the knowledge base
    println!("  Documents indexed: {} (mock embeddings: 4-dim)", DOCS.len());
    let api_key = std::env::var("OPENAI_API_KEY").unwrap_or_default();
    println!("  Auth: {}", if api_key.is_empty() { "simulator mode" } else { "OPENAI_API_KEY" });
    println!("  Listening on http://localhost:3110/api/rag");
    println!("  Upstream: {} (stream: true)", UPSTREAM_URL);
    println!();

    let svc = ServiceProcess::new("rag-basic")
        .emits::<RagQueryEvent>()
        .faults::<RagFault>()
        .manages::<RagIndexState>()
        .prefix("/api")
        .endpoint(Method::POST, "/rag", post(rag_handler));

    VilApp::new("rag-basic-vector-search")
        .port(3110)
        .service(svc)
        .run()
        .await;
}
