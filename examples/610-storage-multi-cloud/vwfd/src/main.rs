// 610-storage-multi-cloud — VWFD mode (pure workflow, no NativeCode handlers needed)

#[tokio::main]
async fn main() {
    vil_vwfd::app("examples/610-storage-multi-cloud/vwfd/workflows", 8080)
        .run()
        .await;
}
