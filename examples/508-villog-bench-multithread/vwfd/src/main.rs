// 508-villog-bench-multithread — VWFD mode
use serde_json::{json, Value};

#[tokio::main]
async fn main() {
    vil_vwfd::app("examples/508-villog-bench-multithread/vwfd/workflows", 3239)
        .native("villog_bench_multithread", |input| {
            // 508-villog-bench-multithread: villog_bench_multithread
            Ok(serde_json::json!({"_handler": "villog_bench_multithread", "input_keys": input.as_object().map(|o| o.keys().collect::<Vec<_>>())}))
        })
        .run()
        .await;
}
