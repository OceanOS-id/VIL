// 501-villog-stdout-dev — VWFD mode
use serde_json::{json, Value};

#[tokio::main]
async fn main() {
    vil_vwfd::app("examples/501-villog-stdout-dev/vwfd/workflows", 3232)
        .native("villog_stdout_demo", |input| {
            // 501-villog-stdout-dev: villog_stdout_demo
            Ok(serde_json::json!({"_handler": "villog_stdout_demo", "input_keys": input.as_object().map(|o| o.keys().collect::<Vec<_>>())}))
        })
        .run()
        .await;
}
