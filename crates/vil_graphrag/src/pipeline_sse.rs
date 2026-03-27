//! SSE pipeline builders for GraphRAG operations.
//!
//! Factory functions that return configured `HttpSinkBuilder` / `HttpSourceBuilder`
//! for graph-enhanced RAG query streaming via `vil_workflow!`.

use vil_sdk::prelude::*;

const SSE_JSON_TAP: &str = "choices[0].delta.content";

/// Build an HTTP sink that accepts GraphRAG queries via POST.
pub fn graphrag_sink(port: u16, path: &str) -> HttpSinkBuilder {
    HttpSinkBuilder::new("GraphRagSink")
        .port(port)
        .path(path)
        .out_port("trigger_out")
        .in_port("response_data_in")
        .ctrl_in_port("response_ctrl_in")
}

/// Build an SSE source for GraphRAG query streaming.
pub fn graphrag_source(upstream_url: &str, model: &str, context: &str) -> HttpSourceBuilder {
    let system = format!(
        "You are a knowledge-graph-powered assistant. Answer using ONLY the provided graph context.\n\nContext:\n{}",
        context
    );

    HttpSourceBuilder::new("GraphRagSource")
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
                { "role": "user", "content": "Answer based on the graph context." }
            ],
            "stream": true
        }))
}
