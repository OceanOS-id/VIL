//! SSE pipeline builders for model registry operations.
//!
//! Factory functions that return configured `HttpSinkBuilder` / `HttpSourceBuilder`
//! for registry event streaming via `vil_workflow!`.

use vil_sdk::prelude::*;

/// Build an HTTP sink that accepts registry requests via POST.
pub fn registry_sink(port: u16, path: &str) -> HttpSinkBuilder {
    HttpSinkBuilder::new("RegistrySink")
        .port(port)
        .path(path)
        .out_port("trigger_out")
        .in_port("response_data_in")
        .ctrl_in_port("response_ctrl_in")
}

/// Build an SSE source for registry event streaming.
pub fn registry_source(upstream_url: &str) -> HttpSourceBuilder {
    HttpSourceBuilder::new("RegistrySource")
        .url(upstream_url)
        .format(HttpFormat::SSE)
        .json_tap("models[*]")
        .in_port("trigger_in")
        .out_port("response_data_out")
        .ctrl_out_port("response_ctrl_out")
}
