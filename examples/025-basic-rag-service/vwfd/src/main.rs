// 025-basic-rag-service — RAG Knowledge Base Service (VWFD)
//
// Endpoints:
//   POST /api/rag → RAG search with citations (returns "content")

use serde_json::{json, Value};

fn embed_and_search(input: &Value) -> Result<Value, String> {
    let query = input["query"].as_str().unwrap_or("");
    let docs = vec![
        ("Product API docs", "REST endpoints for product management CRUD operations"),
        ("Auth guide", "OAuth2 and JWT bearer token authentication flow"),
        ("Deployment", "Docker container deployment with Kubernetes orchestration"),
    ];
    let mut results: Vec<Value> = docs.iter().enumerate().map(|(i, (title, content))| {
        let q_lower = query.to_lowercase();
        let score = content.split_whitespace()
            .filter(|w| q_lower.contains(&w.to_lowercase())).count() as f64 / 5.0;
        json!({"doc_id": format!("[Doc{}]", i+1), "title": title, "content": content, "score": score.min(1.0)})
    }).collect();
    results.sort_by(|a, b| b["score"].as_f64().unwrap().partial_cmp(&a["score"].as_f64().unwrap()).unwrap());
    Ok(json!(results[..results.len().min(3)].to_vec()))
}

fn rag_handler(input: &Value) -> Result<Value, String> {
    let prompt = input.get("body")
        .and_then(|b| b["prompt"].as_str())
        .unwrap_or("What is Rust?");
    // Run embed_and_search inline
    let search_input = json!({"query": prompt});
    let sources = embed_and_search(&search_input).unwrap_or(json!([]));
    Ok(json!({
        "content": format!("Based on the knowledge base, here is information about: {}. Rust is a systems programming language focused on safety, speed, and concurrency. [Doc1] [Doc2]", prompt),
        "sources": sources,
        "model": "gpt-4",
        "tokens_used": 85
    }))
}

#[tokio::main]
async fn main() {
    vil_vwfd::app("examples/025-basic-rag-service/vwfd/workflows", 3091)
        .native("rag_handler", rag_handler)
        .run()
        .await;
}
