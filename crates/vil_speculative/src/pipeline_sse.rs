use vil_sdk::prelude::*;

pub fn speculative_sink(port: u16, path: &str) -> HttpSinkBuilder {
    HttpSinkBuilder::new("SpeculativeDecodeSink")
        .port(port)
        .path(path)
        .out_port("trigger_out")
        .in_port("response_data_in")
        .ctrl_in_port("response_ctrl_in")
}

pub fn speculative_source(upstream_url: &str) -> HttpSourceBuilder {
    HttpSourceBuilder::new("SpeculativeDecodeSource")
        .url(upstream_url)
        .format(HttpFormat::SSE)
        .json_tap("choices[0].delta.content")
        .in_port("trigger_in")
        .out_port("response_data_out")
        .ctrl_out_port("response_ctrl_out")
        .post_json(serde_json::json!({
            "model": "speculative",
            "messages": [{"role": "user", "content": "decode"}],
            "stream": true
        }))
}
