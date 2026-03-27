//! SSE pipeline builders for LLM proxy operations.
use vil_sdk::prelude::*;

pub fn proxy_sink(port: u16, path: &str) -> HttpSinkBuilder {
    HttpSinkBuilder::new("ProxySink")
        .port(port)
        .path(path)
        .out_port("trigger_out")
        .in_port("response_data_in")
        .ctrl_in_port("response_ctrl_in")
}

pub fn proxy_source(upstream_url: &str, model: &str) -> HttpSourceBuilder {
    HttpSourceBuilder::new("ProxySource")
        .url(upstream_url)
        .format(HttpFormat::SSE)
        .json_tap("choices[0].delta.content")
        .in_port("trigger_in")
        .out_port("response_data_out")
        .ctrl_out_port("response_ctrl_out")
        .post_json(serde_json::json!({
            "model": model,
            "messages": [{"role": "user", "content": "proxy"}],
            "stream": true
        }))
}
