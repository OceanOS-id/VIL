use vil_sdk::prelude::*;

pub fn guardrails_sink(port: u16, path: &str) -> HttpSinkBuilder {
    HttpSinkBuilder::new("GuardrailsSink")
        .port(port).path(path)
        .out_port("trigger_out")
        .in_port("response_data_in")
        .ctrl_in_port("response_ctrl_in")
}

pub fn guardrails_source(upstream_url: &str) -> HttpSourceBuilder {
    HttpSourceBuilder::new("GuardrailsSource")
        .url(upstream_url)
        .format(HttpFormat::SSE)
        .json_tap("result")
        .in_port("trigger_in")
        .out_port("response_data_out")
        .ctrl_out_port("response_ctrl_out")
        .post_json(serde_json::json!({
            "text": ""
        }))
}
