// 305 — RAG Guardrail Pipeline (Hybrid: Native + Python WASM + Native)
use serde_json::{json, Value};

fn rag_context_retrieval(input: &Value) -> Result<Value, String> {
    Ok(json!({"context": "Retrieved healthcare documentation...", "docs": 3}))
}
fn guardrail_hallucination_detector(input: &Value) -> Result<Value, String> {
    let text = input["text"].as_str().unwrap_or("");
    let markers = ["I believe", "probably", "might be", "I'm not sure", "as far as I know"];
    let found: Vec<&str> = markers.iter().filter(|m| text.contains(**m)).copied().collect();
    let confidence = 1.0 - (found.len() as f64 * 0.15);
    Ok(json!({"hallucination_markers": found, "confidence": confidence, "status": if confidence > 0.7 { "PASS" } else { "REVIEW" }}))
}

#[tokio::main]
async fn main() {
    vil_vwfd::app("examples/305-rag-guardrail-pipeline/vwfd/workflows", 3114)
        .native("rag_context_retrieval", rag_context_retrieval)               // Native: simple retrieval
        .wasm("guardrail_pii_detector", "examples/305-rag-guardrail-pipeline/vwfd/wasm/python/guardrail_pii_detector.py")  // Python WASM: PII regex
        .native("guardrail_hallucination_detector", guardrail_hallucination_detector)  // Native: marker detection
        .run().await;
}
