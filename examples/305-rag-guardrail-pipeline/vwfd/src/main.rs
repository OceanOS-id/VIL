// 305 — RAG Guardrail Pipeline (VWFD)
// Business logic identical to standard:
//   POST /api/safe-rag — retrieve context → LLM → PII check → hallucination check →
//   status (PASS/REDACTED/BLOCKED) + confidence_score + pii_detections + disclaimer
use serde_json::{json, Value};

fn rag_context_retrieval(input: &Value) -> Result<Value, String> {
    let query = input["query"].as_str().unwrap_or("");
    let results = vec![
        json!({"chunk_id": "HC-001", "text": "Hypertension management: First-line treatment includes ACE inhibitors or ARBs. Target BP <130/80 mmHg for most adults. Lifestyle modifications recommended alongside pharmacotherapy.", "score": 0.89}),
        json!({"chunk_id": "HC-002", "text": "Diabetes monitoring: HbA1c target <7% for most adults. Blood glucose self-monitoring recommended 4-6 times daily for insulin-dependent patients. Annual eye and foot exams required.", "score": 0.76}),
        json!({"chunk_id": "HC-003", "text": "Medication interactions: Metformin contraindicated with eGFR <30. NSAIDs may reduce efficacy of antihypertensives. Always check renal function before prescribing.", "score": 0.71}),
    ];
    let context_text = results.iter()
        .map(|r| r["text"].as_str().unwrap_or("").to_string())
        .collect::<Vec<_>>()
        .join("\n\n");
    Ok(json!({
        "results": results,
        "context_text": context_text,
        "query": query
    }))
}

fn guardrail_hallucination_detector(input: &Value) -> Result<Value, String> {
    let text = input["answer_text"].as_str().unwrap_or("");
    let markers_defs = [
        ("unsupported_claim", &["studies show", "research proves", "it is known that"][..]),
        ("fabricated_reference", &["according to Dr.", "as published in", "per the 2023 study"]),
        ("confidence_without_evidence", &["I am certain", "definitely", "without a doubt"]),
        ("contradiction", &["however this contradicts", "on the other hand"]),
        ("out_of_scope", &["in my opinion", "I believe", "I think"]),
    ];
    let mut found: Vec<String> = Vec::new();
    for (marker, phrases) in &markers_defs {
        for phrase in *phrases {
            if text.to_lowercase().contains(&phrase.to_lowercase()) {
                found.push(marker.to_string());
                break;
            }
        }
    }
    let score = (found.len() as f64 * 0.2).min(1.0);
    let is_hallucinated = score > 0.5;
    Ok(json!({
        "score": score,
        "markers_found": found,
        "is_hallucinated": is_hallucinated
    }))
}

#[tokio::main]
async fn main() {
    vil_vwfd::app("examples/305-rag-guardrail-pipeline/vwfd/workflows", 3114)
        .native("rag_context_retrieval", rag_context_retrieval)
        .sidecar("guardrail_pii_detector", "python3 -u examples/305-rag-guardrail-pipeline/vwfd/sidecar/python/guardrail_pii_detector.py")
        .native("guardrail_hallucination_detector", guardrail_hallucination_detector)
        .run().await;
}
