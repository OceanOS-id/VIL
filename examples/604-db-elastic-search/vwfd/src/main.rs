// 604-db-elastic-search — VWFD mode (pure workflow, no NativeCode handlers needed)

#[tokio::main]
async fn main() {
    vil_vwfd::app("examples/604-db-elastic-search/vwfd/workflows", 3244)
        .run()
        .await;
}
