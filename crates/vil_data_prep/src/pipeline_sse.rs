use vil_sdk::prelude::*;

pub fn data_prep_sink(port: u16, path: &str) -> HttpSinkBuilder {
    HttpSinkBuilder::new("DataPrepSink")
        .port(port).path(path)
        .out_port("trigger_out")
        .in_port("response_data_in")
        .ctrl_in_port("response_ctrl_in")
}

pub fn data_prep_source(upstream_url: &str) -> HttpSourceBuilder {
    HttpSourceBuilder::new("DataPrepSource")
        .url(upstream_url)
        .format(HttpFormat::SSE)
        .json_tap("records")
        .in_port("trigger_in")
        .out_port("response_data_out")
        .ctrl_out_port("response_ctrl_out")
        .post_json(serde_json::json!({
            "texts": [],
            "format": "jsonl"
        }))
}
