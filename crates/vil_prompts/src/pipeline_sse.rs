use vil_sdk::prelude::*;

pub fn prompts_sink(port: u16, path: &str) -> HttpSinkBuilder {
    HttpSinkBuilder::new("PromptsSink")
        .port(port).path(path)
        .out_port("trigger_out")
        .in_port("response_data_in")
        .ctrl_in_port("response_ctrl_in")
}

pub fn prompts_source(upstream_url: &str) -> HttpSourceBuilder {
    HttpSourceBuilder::new("PromptsSource")
        .url(upstream_url)
        .format(HttpFormat::SSE)
        .json_tap("rendered")
        .in_port("trigger_in")
        .out_port("response_data_out")
        .ctrl_out_port("response_ctrl_out")
        .post_json(serde_json::json!({
            "template_name": "rag_qa",
            "variables": {}
        }))
}
