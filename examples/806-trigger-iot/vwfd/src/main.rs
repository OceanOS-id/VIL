// 806-trigger-iot — VWFD mode (pure workflow, no NativeCode handlers needed)

#[tokio::main]
async fn main() {
    vil_vwfd::app("examples/806-trigger-iot/vwfd/workflows", 3263)
        .run()
        .await;
}
