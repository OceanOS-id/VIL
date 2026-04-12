// 507-villog-bench-file-drain — VWFD mode
use serde_json::{json, Value};

#[tokio::main]
async fn main() {
    vil_vwfd::app("examples/507-villog-bench-file-drain/vwfd/workflows", 3238)
        .native("villog_bench_file", |input| {
            // 507-villog-bench-file-drain: villog_bench_file
            Ok(serde_json::json!({"_handler": "villog_bench_file", "input_keys": input.as_object().map(|o| o.keys().collect::<Vec<_>>())}))
        })
        .run()
        .await;
}
