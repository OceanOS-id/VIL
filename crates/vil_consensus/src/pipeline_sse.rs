use vil_sdk::prelude::*;

pub fn consensus_sink(port: u16, path: &str) -> HttpSinkBuilder {
    HttpSinkBuilder::new("ConsensusSink")
        .port(port)
        .path(path)
        .out_port("trigger_out")
        .in_port("response_data_in")
        .ctrl_in_port("response_ctrl_in")
}

pub fn consensus_source(upstream_url: &str) -> HttpSourceBuilder {
    HttpSourceBuilder::new("ConsensusSource")
        .url(upstream_url)
        .format(HttpFormat::SSE)
        .json_tap("choices[0].delta.content")
        .in_port("trigger_in")
        .out_port("response_data_out")
        .ctrl_out_port("response_ctrl_out")
        .post_json(serde_json::json!({
            "model": "consensus",
            "messages": [{"role": "user", "content": "query"}],
            "stream": true
        }))
}
