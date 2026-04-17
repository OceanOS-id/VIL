// 307 — VectorDB Knowledge Index (VWFD)
// Business logic identical to standard:
//   POST /api/search/index — index a document (mock embed + store)
//   POST /api/search/query — HNSW search with keyword scoring
use serde_json::{json, Value};
use std::sync::atomic::{AtomicU64, Ordering};

static DOC_COUNTER: AtomicU64 = AtomicU64::new(1);

fn index_handler(input: &Value) -> Result<Value, String> {
    let body = input.get("body").cloned().unwrap_or(json!({}));
    let title = body["title"].as_str().unwrap_or("Untitled");
    let id = format!("doc_{}", DOC_COUNTER.fetch_add(1, Ordering::Relaxed));
    Ok(json!({
        "id": id,
        "title": title,
        "vector_id": DOC_COUNTER.load(Ordering::Relaxed),
        "dimension": 128
    }))
}

fn query_handler(input: &Value) -> Result<Value, String> {
    let body = input.get("body").cloned().unwrap_or(json!({}));
    let query = body["query"].as_str().or(body["prompt"].as_str()).unwrap_or("");
    let top_k = body["top_k"].as_u64().unwrap_or(3) as usize;

    let docs = vec![
        ("Security best practices", "authentication, authorization, RBAC, JWT tokens"),
        ("Performance tuning", "caching, connection pooling, query optimization"),
        ("Deployment guide", "Docker, Kubernetes, CI/CD pipeline configuration"),
        ("API reference", "REST endpoints, request format, response codes"),
    ];

    let mut results: Vec<Value> = docs.iter().enumerate().map(|(i, (title, keywords))| {
        let words: Vec<&str> = query.split_whitespace().collect();
        let total = words.len().max(1);
        let matched = words.iter().filter(|w| keywords.to_lowercase().contains(&w.to_lowercase())).count();
        let score = matched as f64 / total as f64;
        json!({"doc_id": format!("doc_{}", i+1), "title": title, "score": (score * 100.0).round() / 100.0, "snippet": keywords})
    }).collect();

    results.sort_by(|a, b| b["score"].as_f64().partial_cmp(&a["score"].as_f64()).unwrap_or(std::cmp::Ordering::Equal));
    results.truncate(top_k);

    Ok(json!({"query": query, "top_k": top_k, "results": results}))
}

#[tokio::main]
async fn main() {
    vil_vwfd::app("examples/307-rag-vectordb-knowledge-index/vwfd/workflows", 3107)
        .native("index_handler", index_handler)
        .native("query_handler", query_handler)
        .run().await;
}
