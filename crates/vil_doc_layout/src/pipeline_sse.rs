use vil_sdk::prelude::*;

pub fn layout_sink(port: u16, path: &str) -> HttpSinkBuilder {
    HttpSinkBuilder::new("LayoutSink")
        .port(port).path(path)
        .out_port("trigger_out")
        .in_port("response_data_in")
        .ctrl_in_port("response_ctrl_in")
}

pub fn layout_source(upstream_url: &str) -> HttpSourceBuilder {
    HttpSourceBuilder::new("LayoutSource")
        .url(upstream_url)
        .format(HttpFormat::SSE)
        .json_tap("regions")
        .in_port("trigger_in")
        .out_port("response_data_out")
        .ctrl_out_port("response_ctrl_out")
        .post_json(serde_json::json!({
            "text": ""
        }))
}
