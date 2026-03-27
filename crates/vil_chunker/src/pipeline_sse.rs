use vil_sdk::prelude::*;

pub fn chunker_sink(port: u16, path: &str) -> HttpSinkBuilder {
    HttpSinkBuilder::new("ChunkerSink")
        .port(port).path(path)
        .out_port("trigger_out")
        .in_port("response_data_in")
        .ctrl_in_port("response_ctrl_in")
}

pub fn chunker_source(upstream_url: &str) -> HttpSourceBuilder {
    HttpSourceBuilder::new("ChunkerSource")
        .url(upstream_url)
        .format(HttpFormat::SSE)
        .json_tap("chunks")
        .in_port("trigger_in")
        .out_port("response_data_out")
        .ctrl_out_port("response_ctrl_out")
        .post_json(serde_json::json!({
            "text": "",
            "max_tokens": 512
        }))
}
