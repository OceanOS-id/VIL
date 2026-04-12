// 802-trigger-fs-watcher — VWFD mode (pure workflow, no NativeCode handlers needed)

#[tokio::main]
async fn main() {
    vil_vwfd::app("examples/802-trigger-fs-watcher/vwfd/workflows", 3259)
        .run()
        .await;
}
