// 804-trigger-cdc-postgres — VWFD mode (pure workflow, no NativeCode handlers needed)

#[tokio::main]
async fn main() {
    vil_vwfd::app("examples/804-trigger-cdc-postgres/vwfd/workflows", 3261)
        .run()
        .await;
}
