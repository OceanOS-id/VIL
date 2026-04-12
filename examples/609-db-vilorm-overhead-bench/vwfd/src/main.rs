// 609-db-vilorm-overhead-bench — VWFD mode (pure workflow, no NativeCode handlers needed)

#[tokio::main]
async fn main() {
    vil_vwfd::app("examples/609-db-vilorm-overhead-bench/vwfd/workflows", 3249)
        .run()
        .await;
}
