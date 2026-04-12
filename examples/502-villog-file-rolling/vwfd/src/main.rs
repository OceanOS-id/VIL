// 502-villog-file-rolling — VWFD mode
use serde_json::{json, Value};

#[tokio::main]
async fn main() {
    vil_vwfd::app("examples/502-villog-file-rolling/vwfd/workflows", 3233)
        .native("villog_file_rolling", |input| {
            // 502-villog-file-rolling: villog_file_rolling
            Ok(serde_json::json!({"_handler": "villog_file_rolling", "input_keys": input.as_object().map(|o| o.keys().collect::<Vec<_>>())}))
        })
        .run()
        .await;
}
