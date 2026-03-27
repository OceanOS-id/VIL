//! SSE pipeline builders for workflow orchestration operations.
//!
//! Factory functions that return configured `HttpSinkBuilder` / `HttpSourceBuilder`
//! for workflow execution streaming via `vil_workflow!`.

use vil_sdk::prelude::*;

/// Build an HTTP sink that accepts workflow submissions via POST.
pub fn workflow_sink(port: u16, path: &str) -> HttpSinkBuilder {
    HttpSinkBuilder::new("WorkflowSink")
        .port(port)
        .path(path)
        .out_port("trigger_out")
        .in_port("response_data_in")
        .ctrl_in_port("response_ctrl_in")
}

/// Build an SSE source for workflow execution status streaming.
pub fn workflow_source(upstream_url: &str) -> HttpSourceBuilder {
    HttpSourceBuilder::new("WorkflowSource")
        .url(upstream_url)
        .format(HttpFormat::SSE)
        .json_tap("results[*]")
        .in_port("trigger_in")
        .out_port("response_data_out")
        .ctrl_out_port("response_ctrl_out")
}
