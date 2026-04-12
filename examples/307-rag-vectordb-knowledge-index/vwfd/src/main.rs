use serde_json::{json, Value};
fn vectordb_hnsw_search(input: &Value) -> Result<Value, String> {
    let query = input["query"].as_str().unwrap_or("");
    let docs = vec![
        ("Security best practices", "authentication, authorization, RBAC, JWT tokens"),
        ("Performance tuning", "caching, connection pooling, query optimization"),
        ("Deployment guide", "Docker, Kubernetes, CI/CD pipeline configuration"),
        ("API reference", "REST endpoints, request format, response codes"),
    ];
    let mut results: Vec<Value> = docs.iter().enumerate().map(|(i, (title, keywords))| {
        let score = query.split_whitespace()
            .filter(|w| keywords.to_lowercase().contains(&w.to_lowercase()))
            .count() as f64 / query.split_whitespace().count().max(1) as f64;
        json!({"doc_id": format!("doc_{}", i+1), "title": title, "score": (score * 100.0) as u32, "snippet": keywords})
    }).collect();
    results.sort_by(|a, b| b["score"].as_u64().cmp(&a["score"].as_u64()));
    Ok(json!({"results": &results[..results.len().min(3)], "query": query, "total_searched": docs.len()}))
}
#[tokio::main]
async fn main() {
    vil_vwfd::app("examples/307-rag-vectordb-knowledge-index/vwfd/workflows", 3107)
        .native("vectordb_hnsw_search", vectordb_hnsw_search).run().await;
}
