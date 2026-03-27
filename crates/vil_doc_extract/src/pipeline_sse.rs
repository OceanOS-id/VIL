//! SSE pipeline builders for document extraction operations.
//!
//! Factory functions that return configured `HttpSinkBuilder` / `HttpSourceBuilder`
//! for extraction result streaming via `vil_workflow!`.

use vil_sdk::prelude::*;

/// Build an HTTP sink that accepts extraction requests via POST.
pub fn extract_sink(port: u16, path: &str) -> HttpSinkBuilder {
    HttpSinkBuilder::new("ExtractSink")
        .port(port)
        .path(path)
        .out_port("trigger_out")
        .in_port("response_data_in")
        .ctrl_in_port("response_ctrl_in")
}

/// Build an SSE source for extraction result streaming.
pub fn extract_source(upstream_url: &str) -> HttpSourceBuilder {
    HttpSourceBuilder::new("ExtractSource")
        .url(upstream_url)
        .format(HttpFormat::SSE)
        .json_tap("fields")
        .in_port("trigger_in")
        .out_port("response_data_out")
        .ctrl_out_port("response_ctrl_out")
}
