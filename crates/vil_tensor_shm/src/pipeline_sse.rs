use vil_sdk::prelude::*;

pub fn tensor_stats_sink(port: u16, path: &str) -> HttpSinkBuilder {
    HttpSinkBuilder::new("TensorStatsSink")
        .port(port)
        .path(path)
        .out_port("trigger_out")
        .in_port("response_data_in")
        .ctrl_in_port("response_ctrl_in")
}

pub fn tensor_stats_source(upstream_url: &str) -> HttpSourceBuilder {
    HttpSourceBuilder::new("TensorStatsSource")
        .url(upstream_url)
        .format(HttpFormat::SSE)
        .json_tap("choices[0].delta.content")
        .in_port("trigger_in")
        .out_port("response_data_out")
        .ctrl_out_port("response_ctrl_out")
        .post_json(serde_json::json!({
            "model": "tensor-shm",
            "messages": [{"role": "user", "content": "stats"}],
            "stream": true
        }))
}
