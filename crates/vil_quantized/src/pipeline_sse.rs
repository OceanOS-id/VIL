//! SSE pipeline builders for Quantized Model Runtime operations.
//!
//! Factory functions that return configured `HttpSinkBuilder` / `HttpSourceBuilder`
//! for use with `vil_workflow!` macro.

use vil_sdk::prelude::*;

/// Build an HTTP sink that accepts quantized inference requests via POST.
pub fn quantized_sink(port: u16, path: &str) -> HttpSinkBuilder {
    HttpSinkBuilder::new("QuantizedSink")
        .port(port)
        .path(path)
        .out_port("trigger_out")
        .in_port("response_data_in")
        .ctrl_in_port("response_ctrl_in")
}

/// Build an SSE source that streams quantized inference results.
pub fn quantized_source(upstream_url: &str) -> HttpSourceBuilder {
    HttpSourceBuilder::new("QuantizedSource")
        .url(upstream_url)
        .format(HttpFormat::SSE)
        .json_tap("choices[0].delta.content")
        .in_port("trigger_in")
        .out_port("response_data_out")
        .ctrl_out_port("response_ctrl_out")
        .post_json(serde_json::json!({
            "prompt": "",
            "max_tokens": 256,
            "stream": true
        }))
}
