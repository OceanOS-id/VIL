// 503-villog-multi-drain — VWFD mode
use serde_json::{json, Value};

#[tokio::main]
async fn main() {
    vil_vwfd::app("examples/503-villog-multi-drain/vwfd/workflows", 3234)
        .native("villog_multi_drain", |input| {
            // 503-villog-multi-drain: villog_multi_drain
            Ok(serde_json::json!({"_handler": "villog_multi_drain", "input_keys": input.as_object().map(|o| o.keys().collect::<Vec<_>>())}))
        })
        .run()
        .await;
}
