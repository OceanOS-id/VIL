use vil_sdk::prelude::*;

pub fn query_sink(port: u16, path: &str) -> HttpSinkBuilder {
    HttpSinkBuilder::new("RealtimeRagQuerySink")
        .port(port).path(path)
        .out_port("trigger_out")
        .in_port("response_data_in")
        .ctrl_in_port("response_ctrl_in")
}

pub fn query_source(upstream_url: &str) -> HttpSourceBuilder {
    HttpSourceBuilder::new("RealtimeRagQuerySource")
        .url(upstream_url)
        .format(HttpFormat::SSE)
        .json_tap("choices[0].delta.content")
        .in_port("trigger_in")
        .out_port("response_data_out")
        .ctrl_out_port("response_ctrl_out")
        .post_json(serde_json::json!({
            "model": "realtime-rag",
            "messages": [{"role": "user", "content": "query"}],
            "stream": true
        }))
}
