// 308 — RAG Full Pipeline: Ingest + Query + Stats (VWFD)
// Business logic identical to standard:
//   POST /api/rag/ingest — mock embed + store, return doc_id + chunks_stored
//   POST /api/rag/query — search + LLM generate via Connector
//   GET  /api/rag/stats — document count + query count
use serde_json::{json, Value};
use std::sync::atomic::{AtomicU64, Ordering};

static DOC_COUNT: AtomicU64 = AtomicU64::new(5); // pre-seeded 5 docs
static QUERY_COUNT: AtomicU64 = AtomicU64::new(0);

fn ingest(input: &Value) -> Result<Value, String> {
    let body = input.get("body").cloned().unwrap_or(json!({}));
    let doc_id = body["doc_id"].as_str().unwrap_or("DOC-AUTO");
    DOC_COUNT.fetch_add(1, Ordering::Relaxed);
    Ok(json!({
        "doc_id": doc_id,
        "chunks_stored": 1,
        "embedding_dim": 64
    }))
}

fn stats(_input: &Value) -> Result<Value, String> {
    Ok(json!({
        "total_documents": DOC_COUNT.load(Ordering::Relaxed),
        "embedding_dim": 64,
        "total_queries": QUERY_COUNT.load(Ordering::Relaxed)
    }))
}

#[tokio::main]
async fn main() {
    vil_vwfd::app("examples/308-rag-full-pipeline-ingest-query/vwfd/workflows", 8080)
        .native("ingest_handler", ingest)
        .native("stats_handler", stats)
        .run().await;
}
