// 306 — RAG AI Event Tracking (VWFD)
// Business logic identical to standard:
//   POST /api/support/ask — keyword search 12 articles + LLM via Connector
//   GET  /api/support/quality — quality dashboard (total_queries, avg latency, low_confidence)
use serde_json::{json, Value};
use std::sync::atomic::{AtomicU64, Ordering};

static TOTAL_QUERIES: AtomicU64 = AtomicU64::new(0);
static RETRIEVAL_MS_SUM: AtomicU64 = AtomicU64::new(0);
static GENERATION_MS_SUM: AtomicU64 = AtomicU64::new(0);
static LOW_CONFIDENCE: AtomicU64 = AtomicU64::new(0);

fn quality_handler(_input: &Value) -> Result<Value, String> {
    let total = TOTAL_QUERIES.load(Ordering::Relaxed);
    let ret_sum = RETRIEVAL_MS_SUM.load(Ordering::Relaxed) as f64 / 1000.0;
    let gen_sum = GENERATION_MS_SUM.load(Ordering::Relaxed) as f64 / 1000.0;

    Ok(json!({
        "total_queries": total,
        "avg_retrieval_ms": if total > 0 { ret_sum / total as f64 } else { 0.0 },
        "avg_generation_ms": if total > 0 { gen_sum / total as f64 } else { 0.0 },
        "low_confidence_count": LOW_CONFIDENCE.load(Ordering::Relaxed)
    }))
}

#[tokio::main]
async fn main() {
    vil_vwfd::app("examples/306-rag-ai-event-tracking/vwfd/workflows", 8080)
        .sidecar("rag_keyword_article_search", "python3 -u examples/306-rag-ai-event-tracking/vwfd/sidecar/python/rag_keyword_search.py")
        .native("quality_handler", quality_handler)
        .run().await;
}
