// 304 — RAG Citation Extraction (VWFD)
// Business logic identical to standard:
//   POST /api/cited-rag — retrieve legal docs → LLM with citation instructions →
//   extract [DocN] refs → structured Citation objects
use serde_json::{json, Value};

fn rag_legal_doc_retrieval(input: &Value) -> Result<Value, String> {
    let top_k = input["top_k"].as_u64().unwrap_or(4) as usize;
    let chunks = vec![
        json!({"doc_id": "DOC-2024-001", "title": "Master Service Agreement", "section": "Section 4.2", "page": 12, "content": "Payment terms: net 30 days from invoice date. Late payment incurs 1.5% monthly interest. Client may dispute charges within 15 business days."}),
        json!({"doc_id": "DOC-2024-002", "title": "Amendment #3 — Scope Change", "section": "Section 2.1", "page": 3, "content": "Effective date changed to Q2 2024. Additional deliverables: API integration module and data migration toolkit."}),
        json!({"doc_id": "DOC-2024-003", "title": "Non-Disclosure Agreement", "section": "Section 5.3", "page": 8, "content": "Confidential information includes trade secrets, client lists, and proprietary algorithms. Obligation survives 3 years post-termination."}),
        json!({"doc_id": "DOC-2024-004", "title": "Statement of Work #7", "section": "Section 1.1", "page": 1, "content": "Deliverables: Phase 1 design document (2 weeks), Phase 2 implementation (6 weeks), Phase 3 testing (2 weeks). Total budget: $85,000."}),
    ];
    let selected: Vec<Value> = chunks.into_iter().take(top_k).collect();
    let formatted = selected.iter().enumerate()
        .map(|(i, c)| format!("[Doc{}] ({}, {}): {}", i + 1, c["title"].as_str().unwrap_or(""), c["section"].as_str().unwrap_or(""), c["content"].as_str().unwrap_or("")))
        .collect::<Vec<_>>()
        .join("\n\n");
    let total = selected.len();
    Ok(json!({
        "chunks": selected,
        "formatted_context": formatted,
        "total_chunks": total
    }))
}

#[tokio::main]
async fn main() {
    vil_vwfd::app("examples/304-rag-citation-extraction/vwfd/workflows", 3113)
        .native("rag_legal_doc_retrieval", rag_legal_doc_retrieval)
        .sidecar("rag_citation_extractor", "Rscript examples/304-rag-citation-extraction/vwfd/sidecar/r/citation_extractor.R")
        .run().await;
}
