// 205 — VWFD mode: Legal Document Chunked Summarizer
//
// Workflow: trigger → chunk_splitter (NativeCode) → build prompt → LLM → respond
//
// chunk_splitter splits at sentence boundaries — not expressible in V-CEL.

use serde_json::{json, Value};

/// Split text into chunks at sentence boundaries (., !, ?).
fn split_into_chunks(text: &str, max_size: usize) -> Vec<String> {
    let max_size = max_size.max(100);
    let mut chunks = Vec::new();
    let mut current = String::new();
    for sentence in text.split_inclusive(|c: char| c == '.' || c == '!' || c == '?') {
        if current.len() + sentence.len() > max_size && !current.is_empty() {
            chunks.push(std::mem::take(&mut current));
        }
        current.push_str(sentence);
    }
    if !current.is_empty() { chunks.push(current); }
    if chunks.is_empty() && !text.is_empty() {
        // No sentence boundaries — split at word boundaries
        let mut start = 0;
        while start < text.len() {
            let end = (start + max_size).min(text.len());
            let split = if end < text.len() {
                text[start..end].rfind(' ').map(|p| start + p + 1).unwrap_or(end)
            } else { end };
            chunks.push(text[start..split].to_string());
            start = split;
        }
    }
    chunks
}

/// NativeCode handler: split document text into sentence-boundary chunks.
fn chunk_splitter(input: &Value) -> Result<Value, String> {
    let text = input["text"].as_str().unwrap_or("");
    let max = input["max_chunk_size"].as_u64().unwrap_or(2000) as usize;
    let chunks = split_into_chunks(text, max);
    Ok(json!(chunks))
}

#[tokio::main]
async fn main() {
    vil_vwfd::app("examples/205-llm-chunked-summarizer/vwfd/workflows", 3104)
        .native("chunk_splitter", chunk_splitter)
        .run()
        .await;
}
