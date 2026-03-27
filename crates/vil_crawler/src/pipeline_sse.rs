//! SSE pipeline builders for crawler operations.
//!
//! Factory functions that return configured `HttpSinkBuilder` / `HttpSourceBuilder`
//! for crawler streaming via `vil_workflow!`.

use vil_sdk::prelude::*;

/// Build an HTTP sink that accepts crawl requests via POST.
pub fn crawler_sink(port: u16, path: &str) -> HttpSinkBuilder {
    HttpSinkBuilder::new("CrawlerSink")
        .port(port)
        .path(path)
        .out_port("trigger_out")
        .in_port("response_data_in")
        .ctrl_in_port("response_ctrl_in")
}

/// Build an SSE source for crawler result streaming.
pub fn crawler_source(upstream_url: &str) -> HttpSourceBuilder {
    HttpSourceBuilder::new("CrawlerSource")
        .url(upstream_url)
        .format(HttpFormat::SSE)
        .json_tap("results[*]")
        .in_port("trigger_in")
        .out_port("response_data_out")
        .ctrl_out_port("response_ctrl_out")
}
