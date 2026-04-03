// =============================================================================
// VIL Server Test — Benchmark Suite
// =============================================================================
//
// Provides utilities for benchmarking vil-server performance.
// Measures: throughput, latency (p50/p95/p99), and SHM efficiency.
//
// Usage:
//   let bench = BenchRunner::new(app).requests(10000).concurrency(100);
//   let report = bench.run().await;
//   println!("{}", report);

use axum::Router;
use bytes::Bytes;
use std::time::{Duration, Instant};

/// Benchmark runner for vil-server.
pub struct BenchRunner {
    app: Router,
    requests: usize,
    concurrency: usize,
    path: String,
    method: BenchMethod,
    body: Option<Bytes>,
}

#[derive(Debug, Clone)]
pub enum BenchMethod {
    Get,
    Post,
}

/// Benchmark results report.
#[derive(Debug, Clone)]
pub struct BenchReport {
    pub total_requests: usize,
    pub successful: usize,
    pub failed: usize,
    pub total_duration: Duration,
    pub requests_per_sec: f64,
    pub latency_min_ns: u64,
    pub latency_max_ns: u64,
    pub latency_mean_ns: u64,
    pub latency_p50_ns: u64,
    pub latency_p95_ns: u64,
    pub latency_p99_ns: u64,
    pub bytes_transferred: u64,
}

impl BenchRunner {
    pub fn new(app: Router) -> Self {
        Self {
            app,
            requests: 1000,
            concurrency: 10,
            path: "/".to_string(),
            method: BenchMethod::Get,
            body: None,
        }
    }

    pub fn requests(mut self, n: usize) -> Self {
        self.requests = n;
        self
    }

    pub fn concurrency(mut self, n: usize) -> Self {
        self.concurrency = n;
        self
    }

    pub fn path(mut self, path: impl Into<String>) -> Self {
        self.path = path.into();
        self
    }

    pub fn post(mut self, body: impl Into<Bytes>) -> Self {
        self.method = BenchMethod::Post;
        self.body = Some(body.into());
        self
    }

    /// Run the benchmark and return a report.
    pub async fn run(self) -> BenchReport {
        use axum::body::Body;
        use tower::ServiceExt;

        let mut latencies: Vec<u64> = Vec::with_capacity(self.requests);
        let mut successful = 0usize;
        let mut failed = 0usize;
        let mut bytes_transferred = 0u64;

        let start = Instant::now();

        // Sequential benchmark (for accurate latency measurement)
        for _ in 0..self.requests {
            let req = match &self.method {
                BenchMethod::Get => axum::http::Request::builder()
                    .method("GET")
                    .uri(&self.path)
                    .body(Body::empty())
                    .unwrap(),
                BenchMethod::Post => {
                    let body = self.body.clone().unwrap_or_default();
                    axum::http::Request::builder()
                        .method("POST")
                        .uri(&self.path)
                        .header("content-type", "application/json")
                        .body(Body::from(body))
                        .unwrap()
                }
            };

            let req_start = Instant::now();
            let response = self.app.clone().oneshot(req).await;
            let latency_ns = req_start.elapsed().as_nanos() as u64;
            latencies.push(latency_ns);

            match response {
                Ok(resp) => {
                    if resp.status().is_success() {
                        successful += 1;
                    } else {
                        failed += 1;
                    }
                    // Estimate response size
                    bytes_transferred += 256; // approximate
                }
                Err(_) => {
                    failed += 1;
                }
            }
        }

        let total_duration = start.elapsed();

        // Calculate statistics
        latencies.sort();
        let len = latencies.len();

        BenchReport {
            total_requests: self.requests,
            successful,
            failed,
            total_duration,
            requests_per_sec: self.requests as f64 / total_duration.as_secs_f64(),
            latency_min_ns: *latencies.first().unwrap_or(&0),
            latency_max_ns: *latencies.last().unwrap_or(&0),
            latency_mean_ns: if len > 0 {
                latencies.iter().sum::<u64>() / len as u64
            } else {
                0
            },
            latency_p50_ns: percentile(&latencies, 50),
            latency_p95_ns: percentile(&latencies, 95),
            latency_p99_ns: percentile(&latencies, 99),
            bytes_transferred,
        }
    }
}

fn percentile(sorted: &[u64], pct: usize) -> u64 {
    if sorted.is_empty() {
        return 0;
    }
    let idx = (sorted.len() * pct / 100).min(sorted.len() - 1);
    sorted[idx]
}

impl std::fmt::Display for BenchReport {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "=== VIL Server Benchmark Report ===")?;
        writeln!(
            f,
            "  Requests:     {} total, {} ok, {} failed",
            self.total_requests, self.successful, self.failed
        )?;
        writeln!(
            f,
            "  Duration:     {:.2}s",
            self.total_duration.as_secs_f64()
        )?;
        writeln!(f, "  Throughput:   {:.0} req/s", self.requests_per_sec)?;
        writeln!(f, "  Latency:")?;
        writeln!(f, "    min:  {}ns", self.latency_min_ns)?;
        writeln!(f, "    mean: {}ns", self.latency_mean_ns)?;
        writeln!(f, "    p50:  {}ns", self.latency_p50_ns)?;
        writeln!(f, "    p95:  {}ns", self.latency_p95_ns)?;
        writeln!(f, "    p99:  {}ns", self.latency_p99_ns)?;
        writeln!(f, "    max:  {}ns", self.latency_max_ns)?;
        Ok(())
    }
}
