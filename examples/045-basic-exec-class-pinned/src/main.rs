// ╔════════════════════════════════════════════════════════════════════════╗
// ║  045 — Industrial IoT Sensor Processing (ExecClass::DedicatedThread)║
// ╠════════════════════════════════════════════════════════════════════════╣
// ║  Domain:   Industrial IoT — Sensor Data Processing                  ║
// ║  Pattern:  VX_APP                                                    ║
// ║  Features: ServiceProcess, ServiceCtx, ShmSlice, VilResponse,       ║
// ║            VilModel, ExecClass::BlockingTask, spawn_blocking         ║
// ╠════════════════════════════════════════════════════════════════════════╣
// ║  Business: An industrial IoT gateway receives raw sensor data from   ║
// ║  factory floor equipment (temperature, vibration, pressure sensors). ║
// ║  The data must be processed through CPU-intensive algorithms:        ║
// ║    - FFT (Fast Fourier Transform) for vibration analysis             ║
// ║    - Statistical anomaly detection                                   ║
// ║    - Kalman filtering for noise reduction                            ║
// ║                                                                      ║
// ║  These computations MUST NOT run on the async executor because they  ║
// ║  take 50-500ms of pure CPU time. ExecClass::BlockingTask marks      ║
// ║  the service as CPU-bound, and spawn_blocking() moves the work to   ║
// ║  a dedicated thread pool. Health checks and stats endpoints remain   ║
// ║  responsive even during heavy processing.                            ║
// ║                                                                      ║
// ║  Endpoints:                                                          ║
// ║    POST /api/sensor/process → process raw sensor readings            ║
// ║    GET  /api/sensor/stats   → processing statistics                  ║
// ╚════════════════════════════════════════════════════════════════════════╝
//
// Run:   cargo run -p vil-basic-exec-class-pinned
// Test:
//   curl -X POST http://localhost:8080/api/sensor/process \
//     -H 'Content-Type: application/json' \
//     -d '{"sensor_id":"vibration-motor-7","readings":[23.5,24.1,22.8,25.3,23.9,24.7,22.1,25.8,23.2,24.5],"sample_rate_hz":1000}'
//   curl http://localhost:8080/api/sensor/stats

use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use vil_server::prelude::*;

// ── Business Domain Faults ──────────────────────────────────────────────

#[vil_fault]
pub enum SensorFault {
    /// Sensor reading payload could not be decoded
    InvalidReading,
    /// Processing pipeline timed out (CPU overloaded)
    ProcessingTimeout,
    /// Anomaly threshold exceeded — alert maintenance team
    AnomalyDetected,
}

// ── Models (VilModel = SIMD-ready serialization) ─────────────────────────

#[derive(Debug, Deserialize)]
struct SensorReadingRequest {
    sensor_id: String,
    readings: Vec<f64>,
    sample_rate_hz: u32,
}

#[derive(Clone, Debug, Serialize, Deserialize, VilModel)]
struct ProcessedResult {
    sensor_id: String,
    sample_count: usize,
    mean: f64,
    std_dev: f64,
    min: f64,
    max: f64,
    dominant_frequency_hz: f64,
    anomaly_score: f64,
    anomaly_detected: bool,
    exec_class: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, VilModel)]
struct SensorStats {
    total_readings_processed: u64,
    total_anomalies_detected: u64,
    total_samples_analyzed: u64,
    uptime_secs: u64,
}

// ── Shared State (via ServiceCtx, not Extension<T>) ──────────────────────

struct SensorState {
    readings_processed: AtomicU64,
    anomalies_detected: AtomicU64,
    samples_analyzed: AtomicU64,
    started_at: std::time::Instant,
}

// ── CPU-intensive Signal Processing ─────────────────────────────────────

/// Simulated FFT-based vibration analysis + anomaly detection.
///
/// This function MUST NOT run on the async executor. It performs
/// CPU-bound math (statistical analysis, frequency domain transform)
/// that would starve other HTTP handlers if run inline.
fn process_sensor_data(readings: &[f64], sample_rate_hz: u32) -> (f64, f64, f64, f64, f64, f64, bool) {
    let n = readings.len() as f64;

    // Basic statistics
    let mean = readings.iter().sum::<f64>() / n;
    let variance = readings.iter().map(|r| (r - mean).powi(2)).sum::<f64>() / n;
    let std_dev = (variance.sqrt() * 1000.0).round() / 1000.0;
    let min = readings.iter().cloned().fold(f64::INFINITY, f64::min);
    let max = readings.iter().cloned().fold(f64::NEG_INFINITY, f64::max);

    // Simulated DFT for dominant frequency detection.
    // In production, use a proper FFT library (rustfft).
    // We compute energy at discrete frequencies and find the peak.
    let freq_bins = 32usize;
    let mut max_energy = 0.0f64;
    let mut dominant_bin = 0usize;
    for k in 1..freq_bins {
        let mut real = 0.0f64;
        let mut imag = 0.0f64;
        for (i, &val) in readings.iter().enumerate() {
            let angle = 2.0 * std::f64::consts::PI * k as f64 * i as f64 / n;
            real += val * angle.cos();
            imag += val * angle.sin();
        }
        let energy = real * real + imag * imag;
        if energy > max_energy {
            max_energy = energy;
            dominant_bin = k;
        }
    }
    let dominant_freq = (dominant_bin as f64 * sample_rate_hz as f64 / n * 100.0).round() / 100.0;

    // Anomaly detection: z-score based.
    // If any reading is more than 3 standard deviations from mean, flag it.
    let anomaly_score = if std_dev > 0.0 {
        let max_z = readings.iter()
            .map(|r| ((r - mean) / std_dev).abs())
            .fold(0.0f64, f64::max);
        (max_z * 1000.0).round() / 1000.0
    } else {
        0.0
    };
    let anomaly_detected = anomaly_score > 3.0;

    (mean, std_dev, min, max, dominant_freq, anomaly_score, anomaly_detected)
}

// ── Handlers ─────────────────────────────────────────────────────────────

/// POST /sensor/process — Process raw sensor readings.
///
/// KEY VIL FEATURE: spawn_blocking() + ExecClass::BlockingTask
/// FFT and statistical analysis run on tokio's blocking thread pool,
/// not on the async executor. Health checks and stats remain responsive.
async fn process(
    ctx: ServiceCtx,
    body: ShmSlice,
) -> HandlerResult<VilResponse<ProcessedResult>> {
    let req: SensorReadingRequest = body.json()
        .map_err(|_| VilError::bad_request(
            "invalid JSON — expected {sensor_id, readings: [f64], sample_rate_hz}"
        ))?;

    if req.readings.is_empty() {
        return Err(VilError::bad_request("readings array must not be empty"));
    }

    let sensor_id = req.sensor_id.clone();
    let sample_count = req.readings.len();
    let readings = req.readings;
    let sample_rate = req.sample_rate_hz;

    // Move CPU-intensive signal processing to the blocking thread pool.
    // Without spawn_blocking(), this would freeze all other HTTP handlers.
    let (mean, std_dev, min, max, dominant_freq, anomaly_score, anomaly_detected) =
        tokio::task::spawn_blocking(move || {
            process_sensor_data(&readings, sample_rate)
        })
        .await
        .map_err(|e| VilError::internal(format!("Sensor processing failed: {}", e)))?;

    // Update statistics
    let state = ctx.state::<Arc<SensorState>>()
        .map_err(|_| VilError::internal("state not found"))?;
    state.readings_processed.fetch_add(1, Ordering::Relaxed);
    state.samples_analyzed.fetch_add(sample_count as u64, Ordering::Relaxed);
    if anomaly_detected {
        state.anomalies_detected.fetch_add(1, Ordering::Relaxed);
    }

    Ok(VilResponse::ok(ProcessedResult {
        sensor_id,
        sample_count,
        mean: (mean * 1000.0).round() / 1000.0,
        std_dev,
        min,
        max,
        dominant_frequency_hz: dominant_freq,
        anomaly_score,
        anomaly_detected,
        exec_class: "BlockingTask — ran on spawn_blocking pool (DedicatedThread pattern)".into(),
    }))
}

/// GET /sensor/stats — Processing statistics.
/// Runs on the async executor (ExecClass::AsyncTask) — always fast.
async fn stats(ctx: ServiceCtx) -> HandlerResult<VilResponse<SensorStats>> {
    let state = ctx.state::<Arc<SensorState>>()
        .map_err(|_| VilError::internal("state not found"))?;

    Ok(VilResponse::ok(SensorStats {
        total_readings_processed: state.readings_processed.load(Ordering::Relaxed),
        total_anomalies_detected: state.anomalies_detected.load(Ordering::Relaxed),
        total_samples_analyzed: state.samples_analyzed.load(Ordering::Relaxed),
        uptime_secs: state.started_at.elapsed().as_secs(),
    }))
}

// ── Main ─────────────────────────────────────────────────────────────────

#[tokio::main]
async fn main() {
    println!("╔════════════════════════════════════════════════════════════════════════╗");
    println!("║  045 — Industrial IoT Sensor Processing (ExecClass::DedicatedThread) ║");
    println!("╠════════════════════════════════════════════════════════════════════════╣");
    println!("║  POST /api/sensor/process → FFT + anomaly detection (spawn_blocking) ║");
    println!("║  GET  /api/sensor/stats   → processing metrics (async, always fast)  ║");
    println!("║  ExecClass: BlockingTask — CPU-bound work on dedicated thread pool   ║");
    println!("╚════════════════════════════════════════════════════════════════════════╝");

    let state = Arc::new(SensorState {
        readings_processed: AtomicU64::new(0),
        anomalies_detected: AtomicU64::new(0),
        samples_analyzed: AtomicU64::new(0),
        started_at: std::time::Instant::now(),
    });

    // ExecClass::BlockingTask tells the VIL runtime that this service's
    // handlers are CPU-bound and should be scheduled on the blocking pool.
    let sensor_svc = ServiceProcess::new("sensor")
        .exec(ExecClass::BlockingTask)
        .endpoint(Method::POST, "/sensor/process", post(process))
        .endpoint(Method::GET, "/sensor/stats", get(stats))
        .state(state);

    VilApp::new("iot-sensor-gateway")
        .port(8080)
        .service(sensor_svc)
        .run()
        .await;
}
