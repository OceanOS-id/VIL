// 016-basic-ai-rag-gateway — RAG Gateway (VWFD)
//
// Endpoints:
//   POST /rag → RAG search + LLM answer

use serde_json::{json, Value};

fn vector_similarity_search(input: &Value) -> Result<Value, String> {
    let query = input["query"].as_str().unwrap_or("");
    let docs = vec![
        json!({"id": "doc1", "title": "API Authentication", "content": "Use Bearer tokens for API auth...", "score": 0.0}),
        json!({"id": "doc2", "title": "Rate Limiting", "content": "Configure rate limits per tenant...", "score": 0.0}),
        json!({"id": "doc3", "title": "Error Handling", "content": "Return structured error responses...", "score": 0.0}),
    ];
    let mut scored: Vec<Value> = docs.into_iter().map(|mut d| {
        let content = d["content"].as_str().unwrap_or("");
        let words: Vec<&str> = query.split_whitespace().collect();
        let score = words.iter().filter(|w| content.to_lowercase().contains(&w.to_lowercase())).count() as f64 / words.len().max(1) as f64;
        d["score"] = json!(score);
        d
    }).collect();
    scored.sort_by(|a, b| b["score"].as_f64().unwrap_or(0.0).partial_cmp(&a["score"].as_f64().unwrap_or(0.0)).unwrap());
    Ok(json!({"results": &scored[..scored.len().min(2)], "query": query}))
}

fn rag_handler(input: &Value) -> Result<Value, String> {
    let prompt = input.get("body")
        .and_then(|b| b["prompt"].as_str())
        .unwrap_or("What is Rust?");
    let search_input = json!({"query": prompt});
    let sources = vector_similarity_search(&search_input).unwrap_or(json!({}));
    Ok(json!({
        "query": prompt,
        "answer": format!("Based on the knowledge base: {}. Rust is a systems programming language focused on safety and performance.", prompt),
        "sources": sources
    }))
}

#[tokio::main]
async fn main() {
    vil_vwfd::app("examples/016-basic-ai-rag-gateway/vwfd/workflows", 3084)
        .native("rag_handler", rag_handler)
        .run().await;
}
