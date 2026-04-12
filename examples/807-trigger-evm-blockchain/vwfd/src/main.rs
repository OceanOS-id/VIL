// 807-trigger-evm-blockchain — VWFD mode (pure workflow, no NativeCode handlers needed)

#[tokio::main]
async fn main() {
    vil_vwfd::app("examples/807-trigger-evm-blockchain/vwfd/workflows", 3264)
        .run()
        .await;
}
