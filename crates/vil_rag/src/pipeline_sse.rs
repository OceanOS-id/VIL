//! SSE pipeline builders for RAG operations.
//!
//! Factory functions that return configured `HttpSinkBuilder` / `HttpSourceBuilder`
//! for RAG query streaming via `vil_workflow!`.
//!
//! # Example
//!
//! ```rust,ignore
//! use vil_sdk::prelude::*;
//! use vil_rag::pipeline_sse;
//!
//! let sink = pipeline_sse::rag_sink(3091, "/rag");
//! let source = pipeline_sse::rag_source(
//!     "http://localhost:4545/v1/chat/completions", "gpt-4",
//!     &["Rust is a systems language.", "VIL is process-oriented."],
//! );
//!
//! let (_ir, (sink_h, source_h)) = vil_workflow! {
//!     name: "RagPipeline",
//!     instances: [ sink, source ],
//!     routes: [
//!         sink.trigger_out -> source.trigger_in (LoanWrite),
//!         source.response_data_out -> sink.response_data_in (LoanWrite),
//!         source.response_ctrl_out -> sink.response_ctrl_in (Copy),
//!     ]
//! };
//! ```

use vil_sdk::prelude::*;

const SSE_JSON_TAP: &str = "choices[0].delta.content";

/// Build an HTTP sink that accepts RAG queries via POST.
pub fn rag_sink(port: u16, path: &str) -> HttpSinkBuilder {
    HttpSinkBuilder::new("RagSink")
        .port(port)
        .path(path)
        .out_port("trigger_out")
        .in_port("response_data_in")
        .ctrl_in_port("response_ctrl_in")
}

/// Build an SSE source for RAG query streaming.
///
/// The system prompt includes retrieved context documents so the LLM
/// generates answers grounded in the provided context.
pub fn rag_source(upstream_url: &str, model: &str, context_docs: &[&str]) -> HttpSourceBuilder {
    let context = context_docs
        .iter()
        .enumerate()
        .map(|(i, doc)| format!("[Doc{}] {}", i + 1, doc))
        .collect::<Vec<_>>()
        .join("\n\n");

    let system = format!(
        "You are a RAG-powered knowledge assistant. Answer questions using ONLY the provided context. \
         Always cite which document [DocN] you reference.\n\nContext:\n{}",
        context
    );

    HttpSourceBuilder::new("RagSource")
        .url(upstream_url)
        .format(HttpFormat::SSE)
        .json_tap(SSE_JSON_TAP)
        .in_port("trigger_in")
        .out_port("response_data_out")
        .ctrl_out_port("response_ctrl_out")
        .post_json(serde_json::json!({
            "model": model,
            "messages": [
                { "role": "system", "content": system },
                { "role": "user", "content": "Answer based on the context documents." }
            ],
            "stream": true
        }))
}
