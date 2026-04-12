// 601-storage-s3-basic — VWFD mode (pure workflow, no NativeCode handlers needed)

#[tokio::main]
async fn main() {
    vil_vwfd::app("examples/601-storage-s3-basic/vwfd/workflows", 3241)
        .run()
        .await;
}
