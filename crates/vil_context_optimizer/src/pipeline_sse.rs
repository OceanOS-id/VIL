//! SSE pipeline builders for Context Optimizer operations.
//!
//! Factory functions that return configured `HttpSinkBuilder` / `HttpSourceBuilder`
//! for use with `vil_workflow!` macro.

use vil_sdk::prelude::*;

/// Build an HTTP sink that accepts optimization requests via POST.
pub fn optimizer_sink(port: u16, path: &str) -> HttpSinkBuilder {
    HttpSinkBuilder::new("OptimizerSink")
        .port(port)
        .path(path)
        .out_port("trigger_out")
        .in_port("response_data_in")
        .ctrl_in_port("response_ctrl_in")
}

/// Build an SSE source that streams optimization results.
pub fn optimizer_source(upstream_url: &str) -> HttpSourceBuilder {
    HttpSourceBuilder::new("OptimizerSource")
        .url(upstream_url)
        .format(HttpFormat::SSE)
        .json_tap("result.chunks")
        .in_port("trigger_in")
        .out_port("response_data_out")
        .ctrl_out_port("response_ctrl_out")
        .post_json(serde_json::json!({
            "chunks": [],
            "budget": 8000,
            "stream": true
        }))
}
