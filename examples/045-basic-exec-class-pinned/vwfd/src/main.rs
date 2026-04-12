// 045 — IoT Sensor Processing (Hybrid: WASM C for FFT computation, NativeCode for stats)
use serde_json::json;

#[tokio::main]
async fn main() {
    vil_vwfd::app("examples/045-basic-exec-class-pinned/vwfd/workflows", 8080)
        // FFT sensor processing — WASM C (CPU-bound DFT, sandboxed execution)
        .wasm("sensor_process", "examples/045-basic-exec-class-pinned/vwfd/wasm/c/iot_fft.wasm")
        // Stats endpoint — NativeCode (simple counter)
        .native("sensor_stats", |_| {
            Ok(json!({"total_processed": 0, "anomalies_detected": 0, "uptime_secs": 0}))
        })
        .run().await;
}
