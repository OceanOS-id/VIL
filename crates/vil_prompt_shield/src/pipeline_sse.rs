//! SSE pipeline builders for Prompt Shield operations.
//!
//! Factory functions that return configured `HttpSinkBuilder` / `HttpSourceBuilder`
//! for use with `vil_workflow!` macro.

use vil_sdk::prelude::*;

/// Build an HTTP sink that accepts shield scan requests via POST.
pub fn shield_sink(port: u16, path: &str) -> HttpSinkBuilder {
    HttpSinkBuilder::new("ShieldSink")
        .port(port)
        .path(path)
        .out_port("trigger_out")
        .in_port("response_data_in")
        .ctrl_in_port("response_ctrl_in")
}

/// Build an SSE source that streams shield scan results.
pub fn shield_source(upstream_url: &str) -> HttpSourceBuilder {
    HttpSourceBuilder::new("ShieldSource")
        .url(upstream_url)
        .format(HttpFormat::SSE)
        .json_tap("result.safe")
        .in_port("trigger_in")
        .out_port("response_data_out")
        .ctrl_out_port("response_ctrl_out")
        .post_json(serde_json::json!({
            "text": "",
            "stream": true
        }))
}
