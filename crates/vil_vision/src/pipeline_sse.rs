//! SSE pipeline builders for vision analysis operations.
//!
//! Factory functions that return configured `HttpSinkBuilder` / `HttpSourceBuilder`
//! for vision analysis streaming via `vil_workflow!`.

use vil_sdk::prelude::*;

/// Build an HTTP sink that accepts image analysis requests via POST.
pub fn vision_sink(port: u16, path: &str) -> HttpSinkBuilder {
    HttpSinkBuilder::new("VisionSink")
        .port(port)
        .path(path)
        .out_port("trigger_out")
        .in_port("response_data_in")
        .ctrl_in_port("response_ctrl_in")
}

/// Build an SSE source for vision analysis streaming.
pub fn vision_source(upstream_url: &str) -> HttpSourceBuilder {
    HttpSourceBuilder::new("VisionSource")
        .url(upstream_url)
        .format(HttpFormat::SSE)
        .json_tap("description")
        .in_port("trigger_in")
        .out_port("response_data_out")
        .ctrl_out_port("response_ctrl_out")
}
