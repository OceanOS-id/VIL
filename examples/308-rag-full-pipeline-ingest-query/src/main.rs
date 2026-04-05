// ╔════════════════════════════════════════════════════════════╗
// ║  308 — Legal Contract RAG (Full Ingest + Query Pipeline)  ║
// ╠════════════════════════════════════════════════════════════╣
// ║  Domain:   Legal Tech — Contract Analysis                  ║
// ║  Pattern:  VX_APP                                           ║
// ║  Features: vil_vectordb::Collection, HNSW search,         ║
// ║            LlmProvider for generation, full RAG cycle      ║
// ╠════════════════════════════════════════════════════════════╣
// ║  Business: Ingest legal contracts → chunk → embed (mock)  ║
// ║  → store in HNSW index → query with citation.              ║
// ║  Full RAG cycle: ingest + retrieve + generate.             ║
// ╚════════════════════════════════════════════════════════════╝
//
// Requires: ai-endpoint-simulator at localhost:4545
//
// Run:   cargo run -p vil-rag-full-pipeline-ingest-query
// Test:
//   curl -X POST http://localhost:8080/api/rag/query \
//     -H 'Content-Type: application/json' \
//     -d '{"question":"What is the termination clause?"}'
//   curl http://localhost:8080/api/rag/stats

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use vil_llm::{ChatMessage, LlmProvider, OpenAiConfig, OpenAiProvider};
use vil_server::prelude::*;
use vil_vectordb::{Collection, HnswConfig};

const EMBEDDING_DIM: usize = 64;

// ── Mock embedding ───────────────────────────────────────────────────────

fn mock_embed(text: &str) -> Vec<f32> {
    let lower = text.to_lowercase();
    let words: Vec<&str> = lower.split_whitespace().collect();
    let mut vec = vec![0.0f32; EMBEDDING_DIM];
    for word in &words {
        let mut hasher = DefaultHasher::new();
        word.hash(&mut hasher);
        let h = hasher.finish();
        let idx = (h as usize) % EMBEDDING_DIM;
        vec[idx] += 1.0;
    }
    let norm: f32 = vec.iter().map(|x| x * x).sum::<f32>().sqrt().max(1e-8);
    vec.iter().map(|x| x / norm).collect()
}

// ── Legal contract sections ──────────────────────────────────────────────

struct ContractSection {
    id: &'static str,
    title: &'static str,
    content: &'static str,
}

const CONTRACTS: &[ContractSection] = &[
    ContractSection { id: "SEC-001", title: "Termination Clause",
        content: "Either party may terminate this Agreement with 30 days written notice. Upon termination, all confidential materials must be returned within 10 business days. Early termination without cause requires payment of 3 months fees as liquidated damages." },
    ContractSection { id: "SEC-002", title: "Limitation of Liability",
        content: "Neither party shall be liable for indirect, incidental, or consequential damages. Total liability under this Agreement shall not exceed the fees paid in the 12 months preceding the claim. This limitation does not apply to breach of confidentiality or IP infringement." },
    ContractSection { id: "SEC-003", title: "Confidentiality",
        content: "Each party agrees to maintain the confidentiality of all proprietary information for 5 years after termination. Confidential information includes trade secrets, customer lists, pricing, and technical specifications. Disclosure is permitted only to employees with need-to-know who are bound by similar obligations." },
    ContractSection { id: "SEC-004", title: "Payment Terms",
        content: "Invoices are due within 30 days of receipt. Late payments accrue interest at 1.5% per month. Client shall reimburse reasonable expenses pre-approved in writing. Annual fee adjustments not to exceed CPI plus 3%." },
    ContractSection { id: "SEC-005", title: "Intellectual Property",
        content: "All work product created during the engagement shall be owned by Client upon full payment. Contractor retains rights to pre-existing tools and methodologies. Client grants Contractor a license to use deliverables for portfolio purposes with written consent." },
];

// ── Models ───────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
struct IngestRequest {
    doc_id: String,
    title: String,
    content: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, VilModel)]
struct IngestResponse {
    doc_id: String,
    chunks_stored: usize,
    embedding_dim: usize,
}

#[derive(Debug, Deserialize)]
struct QueryRequest {
    question: String,
    #[serde(default = "default_top_k")]
    top_k: usize,
}
fn default_top_k() -> usize { 3 }

#[derive(Clone, Debug, Serialize, Deserialize, VilModel)]
struct QueryResponse {
    answer: String,
    sources: Vec<SourceRef>,
    retrieval_ms: f64,
    generation_ms: f64,
}

#[derive(Clone, Debug, Serialize, Deserialize, VilModel)]
struct SourceRef {
    doc_id: String,
    score: f32,
    excerpt: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, VilModel)]
struct StatsResponse {
    total_documents: usize,
    embedding_dim: usize,
    total_queries: u64,
}

struct AppState {
    collection: Collection,
    llm: Arc<dyn LlmProvider>,
    query_count: AtomicU64,
}

// ── Handlers ─────────────────────────────────────────────────────────────

async fn ingest(ctx: ServiceCtx, body: ShmSlice) -> HandlerResult<VilResponse<IngestResponse>> {
    let req: IngestRequest = body.json().map_err(|_| VilError::bad_request("invalid JSON"))?;
    let state = ctx.state::<Arc<AppState>>().map_err(|_| VilError::internal("state"))?;

    let embedding = mock_embed(&format!("{} {}", req.title, req.content));
    let meta = serde_json::json!({"title": req.title, "doc_id": req.doc_id});
    state.collection.add(embedding, meta, Some(req.content.clone()));

    Ok(VilResponse::ok(IngestResponse {
        doc_id: req.doc_id,
        chunks_stored: 1,
        embedding_dim: EMBEDDING_DIM,
    }))
}

async fn query(ctx: ServiceCtx, body: ShmSlice) -> HandlerResult<VilResponse<QueryResponse>> {
    let req: QueryRequest = body.json().map_err(|_| VilError::bad_request("invalid JSON"))?;
    let state = ctx.state::<Arc<AppState>>().map_err(|_| VilError::internal("state"))?;
    state.query_count.fetch_add(1, Ordering::Relaxed);

    // Retrieve
    let ret_start = std::time::Instant::now();
    let query_vec = mock_embed(&req.question);
    let results = state.collection.search(&query_vec, req.top_k);
    let retrieval_ms = ret_start.elapsed().as_secs_f64() * 1000.0;

    let sources: Vec<SourceRef> = results.iter().map(|r| {
        SourceRef {
            doc_id: r.metadata["doc_id"].as_str().unwrap_or("?").into(),
            score: r.score,
            excerpt: r.text.as_deref().unwrap_or("").chars().take(100).collect(),
        }
    }).collect();

    let context = results.iter()
        .map(|r| format!("[{}] {}", r.metadata["doc_id"].as_str().unwrap_or("?"),
                         r.text.as_deref().unwrap_or("")))
        .collect::<Vec<_>>()
        .join("\n\n");

    // Generate
    let gen_start = std::time::Instant::now();
    let messages = vec![
        ChatMessage::system(&format!(
            "You are a legal contract analyst. Answer based on these contract sections. Cite [SEC-NNN].\n\n{}", context
        )),
        ChatMessage::user(&req.question),
    ];
    let resp = state.llm.chat(&messages).await
        .map_err(|e| VilError::internal(format!("LLM: {}", e)))?;
    let generation_ms = gen_start.elapsed().as_secs_f64() * 1000.0;

    Ok(VilResponse::ok(QueryResponse {
        answer: resp.content,
        sources,
        retrieval_ms,
        generation_ms,
    }))
}

async fn stats(ctx: ServiceCtx) -> HandlerResult<VilResponse<StatsResponse>> {
    let state = ctx.state::<Arc<AppState>>().map_err(|_| VilError::internal("state"))?;
    Ok(VilResponse::ok(StatsResponse {
        total_documents: state.collection.count(),
        embedding_dim: EMBEDDING_DIM,
        total_queries: state.query_count.load(Ordering::Relaxed),
    }))
}

#[tokio::main]
async fn main() {
    let upstream = std::env::var("LLM_UPSTREAM").unwrap_or_else(|_| "http://127.0.0.1:4545".into());
    let api_key = std::env::var("OPENAI_API_KEY").unwrap_or_default();

    let llm: Arc<dyn LlmProvider> = Arc::new(OpenAiProvider::new(
        OpenAiConfig::new(&api_key, "gpt-4").base_url(&format!("{}/v1", upstream)),
    ));

    let collection = Collection::new("legal-contracts", EMBEDDING_DIM, HnswConfig::default());

    // Pre-seed legal contract sections
    for sec in CONTRACTS {
        let embedding = mock_embed(&format!("{} {}", sec.title, sec.content));
        let meta = serde_json::json!({"title": sec.title, "doc_id": sec.id});
        collection.add(embedding, meta, Some(sec.content.into()));
    }
    println!("Indexed {} legal contract sections", CONTRACTS.len());

    let state = Arc::new(AppState { collection, llm, query_count: AtomicU64::new(0) });

    let svc = ServiceProcess::new("rag")
        .endpoint(Method::POST, "/ingest", post(ingest))
        .endpoint(Method::POST, "/query", post(query))
        .endpoint(Method::GET, "/stats", get(stats))
        .state(state);

    VilApp::new("legal-rag-pipeline")
        .port(8080)
        .observer(true)
        .service(svc)
        .run()
        .await;
}
