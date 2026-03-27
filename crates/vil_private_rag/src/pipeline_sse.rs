use vil_sdk::prelude::*;

pub fn private_rag_sink(port: u16, path: &str) -> HttpSinkBuilder {
    HttpSinkBuilder::new("PrivateRagSink")
        .port(port).path(path)
        .out_port("trigger_out")
        .in_port("response_data_in")
        .ctrl_in_port("response_ctrl_in")
}

pub fn private_rag_source(upstream_url: &str) -> HttpSourceBuilder {
    HttpSourceBuilder::new("PrivateRagSource")
        .url(upstream_url)
        .format(HttpFormat::SSE)
        .json_tap("redacted")
        .in_port("trigger_in")
        .out_port("response_data_out")
        .ctrl_out_port("response_ctrl_out")
        .post_json(serde_json::json!({
            "text": "",
            "redact": true,
            "anonymize": true
        }))
}
