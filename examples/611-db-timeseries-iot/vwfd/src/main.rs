// 611-db-timeseries-iot — VWFD mode (pure workflow, no NativeCode handlers needed)

#[tokio::main]
async fn main() {
    vil_vwfd::app("examples/611-db-timeseries-iot/vwfd/workflows", 8080)
        .run()
        .await;
}
