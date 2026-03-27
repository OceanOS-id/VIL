//! SSE pipeline builders for A/B testing operations.
//!
//! Factory functions that return configured `HttpSinkBuilder` / `HttpSourceBuilder`
//! for experiment result streaming via `vil_workflow!`.

use vil_sdk::prelude::*;

/// Build an HTTP sink that accepts A/B test requests via POST.
pub fn abtest_sink(port: u16, path: &str) -> HttpSinkBuilder {
    HttpSinkBuilder::new("AbTestSink")
        .port(port)
        .path(path)
        .out_port("trigger_out")
        .in_port("response_data_in")
        .ctrl_in_port("response_ctrl_in")
}

/// Build an SSE source for A/B test result streaming.
pub fn abtest_source(upstream_url: &str) -> HttpSourceBuilder {
    HttpSourceBuilder::new("AbTestSource")
        .url(upstream_url)
        .format(HttpFormat::SSE)
        .json_tap("results")
        .in_port("trigger_in")
        .out_port("response_data_out")
        .ctrl_out_port("response_ctrl_out")
}
