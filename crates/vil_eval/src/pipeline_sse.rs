use vil_sdk::prelude::*;

pub fn eval_sink(port: u16, path: &str) -> HttpSinkBuilder {
    HttpSinkBuilder::new("EvalSink")
        .port(port)
        .path(path)
        .out_port("trigger_out")
        .in_port("response_data_in")
        .ctrl_in_port("response_ctrl_in")
}

pub fn eval_source(upstream_url: &str) -> HttpSourceBuilder {
    HttpSourceBuilder::new("EvalSource")
        .url(upstream_url)
        .format(HttpFormat::SSE)
        .json_tap("report")
        .in_port("trigger_in")
        .out_port("response_data_out")
        .ctrl_out_port("response_ctrl_out")
        .post_json(serde_json::json!({
            "dataset_json": "{}",
            "metrics": ["answer_relevance"]
        }))
}
