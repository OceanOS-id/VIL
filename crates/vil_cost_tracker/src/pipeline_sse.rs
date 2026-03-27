//! SSE pipeline builders for cost tracking operations.
//!
//! Factory functions that return configured `HttpSinkBuilder` / `HttpSourceBuilder`
//! for cost report streaming via `vil_workflow!`.

use vil_sdk::prelude::*;

/// Build an HTTP sink that accepts cost tracking requests via POST.
pub fn cost_sink(port: u16, path: &str) -> HttpSinkBuilder {
    HttpSinkBuilder::new("CostSink")
        .port(port)
        .path(path)
        .out_port("trigger_out")
        .in_port("response_data_in")
        .ctrl_in_port("response_ctrl_in")
}

/// Build an SSE source for cost report streaming.
pub fn cost_source(upstream_url: &str) -> HttpSourceBuilder {
    HttpSourceBuilder::new("CostSource")
        .url(upstream_url)
        .format(HttpFormat::SSE)
        .json_tap("models[*]")
        .in_port("trigger_in")
        .out_port("response_data_out")
        .ctrl_out_port("response_ctrl_out")
}
