// 304 — Citation Extraction (Hybrid: Native doc retrieval + R Sidecar citation extractor)
use serde_json::{json, Value};
fn rag_legal_doc_retrieval(input: &Value) -> Result<Value, String> {
    Ok(json!([
        {"doc_id": "[Doc1]", "title": "Contract §4.2", "content": "Payment terms: net 30 days..."},
        {"doc_id": "[Doc2]", "title": "Amendment #3", "content": "Effective date changed to..."},
    ]))
}
#[tokio::main]
async fn main() {
    vil_vwfd::app("examples/304-rag-citation-extraction/vwfd/workflows", 3113)
        .native("rag_legal_doc_retrieval", rag_legal_doc_retrieval)
        .sidecar("rag_citation_extractor", "Rscript examples/304-rag-citation-extraction/vwfd/sidecar/r/citation_extractor.R")
        .run().await;
}
