// =============================================================================
// VIL Pipeline SSE — Streaming RAG
// =============================================================================

use vil_sdk::prelude::*;

/// Creates an HTTP sink that accepts streaming RAG queries.
pub fn streaming_rag_sink(port: u16, path: &str) -> HttpSinkBuilder {
    HttpSinkBuilder::new("StreamingRagSink")
        .port(port)
        .path(path)
        .out_port("trigger_out")
        .in_port("response_data_in")
        .ctrl_in_port("response_ctrl_in")
}

/// Creates an HTTP source that streams RAG results via SSE.
pub fn streaming_rag_source(upstream_url: &str) -> HttpSourceBuilder {
    HttpSourceBuilder::new("StreamingRagSource")
        .url(upstream_url)
        .format(HttpFormat::SSE)
        .json_tap("results")
        .in_port("trigger_in")
        .out_port("response_data_out")
        .ctrl_out_port("response_ctrl_out")
        .post_json(serde_json::json!({
            "action": "stream_query",
            "stream": true
        }))
}
