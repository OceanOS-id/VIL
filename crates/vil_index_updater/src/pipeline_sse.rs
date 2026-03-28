use vil_sdk::prelude::*;

pub fn index_updater_sink(port: u16, path: &str) -> HttpSinkBuilder {
    HttpSinkBuilder::new("IndexUpdaterSink")
        .port(port)
        .path(path)
        .out_port("trigger_out")
        .in_port("response_data_in")
        .ctrl_in_port("response_ctrl_in")
}

pub fn index_updater_source(upstream_url: &str) -> HttpSourceBuilder {
    HttpSourceBuilder::new("IndexUpdaterSource")
        .url(upstream_url)
        .format(HttpFormat::SSE)
        .json_tap("flush_result")
        .in_port("trigger_in")
        .out_port("response_data_out")
        .ctrl_out_port("response_ctrl_out")
        .post_json(serde_json::json!({
            "operation": "flush"
        }))
}
