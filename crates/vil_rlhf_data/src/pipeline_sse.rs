use vil_sdk::prelude::*;

pub fn rlhf_sink(port: u16, path: &str) -> HttpSinkBuilder {
    HttpSinkBuilder::new("RlhfSink")
        .port(port)
        .path(path)
        .out_port("trigger_out")
        .in_port("response_data_in")
        .ctrl_in_port("response_ctrl_in")
}

pub fn rlhf_source(upstream_url: &str) -> HttpSourceBuilder {
    HttpSourceBuilder::new("RlhfSource")
        .url(upstream_url)
        .format(HttpFormat::SSE)
        .json_tap("pairs")
        .in_port("trigger_in")
        .out_port("response_data_out")
        .ctrl_out_port("response_ctrl_out")
        .post_json(serde_json::json!({
            "format": "dpo"
        }))
}
