// =============================================================================
// VIL Pipeline SSE — AI Gateway
// =============================================================================

use vil_sdk::prelude::*;

/// Creates an HTTP sink that accepts AI gateway chat requests.
pub fn gateway_chat_sink(port: u16, path: &str) -> HttpSinkBuilder {
    HttpSinkBuilder::new("GatewayChatSink")
        .port(port)
        .path(path)
        .out_port("trigger_out")
        .in_port("response_data_in")
        .ctrl_in_port("response_ctrl_in")
}

/// Creates an HTTP source that streams LLM chat completions via SSE.
pub fn gateway_chat_source(upstream_url: &str) -> HttpSourceBuilder {
    HttpSourceBuilder::new("GatewayChatSource")
        .url(upstream_url)
        .format(HttpFormat::SSE)
        .json_tap("choices[0].delta.content")
        .in_port("trigger_in")
        .out_port("response_data_out")
        .ctrl_out_port("response_ctrl_out")
        .post_json(serde_json::json!({
            "model": "auto",
            "messages": [{"role": "user", "content": "hello"}],
            "stream": true
        }))
}
