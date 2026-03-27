use anyhow::Result;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::sync::Arc;
use std::time::{Duration, Instant};

const MOCK_PORT: u16 = 18081;

pub fn run_demo(port: u16) -> Result<()> {
    println!("  Listening on http://localhost:{}/trigger", port);
    println!("  Health check: http://localhost:{}/health", port);
    println!("  Readiness: http://localhost:{}/ready", port);
    println!("  Metrics: http://localhost:{}/metrics", port);
    println!("  Press Ctrl+C to stop\n");
    println!("Test with:");
    println!("  curl -N -X POST -H 'Content-Type: application/json' \\");
    println!(
        "    -d '{{\"request\": \"stream-credits\"}}' http://localhost:{}/trigger\n",
        port
    );

    let start_time = Instant::now();
    let metrics = vil_obs::VilMetrics::new();

    let addr = format!("0.0.0.0:{}", port);
    let listener = TcpListener::bind(&addr)?;

    for stream in listener.incoming() {
        match stream {
            Ok(mut stream) => {
                let mut buffer = [0u8; 2048];
                let n = stream.read(&mut buffer).unwrap_or(0);
                let request = String::from_utf8_lossy(&buffer[..n]);

                let (status, content_type, body) = route_request(&request, &metrics, start_time);

                let response = format!(
                    "HTTP/1.1 {}\r\nContent-Type: {}\r\nContent-Length: {}\r\n\r\n{}",
                    status,
                    content_type,
                    body.len(),
                    body
                );
                let _ = stream.write_all(response.as_bytes());
            }
            Err(e) => eprintln!("Connection error: {}", e),
        }
    }
    Ok(())
}

fn route_request(
    request: &str,
    metrics: &vil_obs::VilMetrics,
    start_time: Instant,
) -> (&'static str, &'static str, String) {
    let first_line = request.lines().next().unwrap_or("");

    if first_line.contains("/health") {
        (
            "200 OK",
            "application/json",
            r#"{"status":"healthy","service":"vil"}"#.to_string(),
        )
    } else if first_line.contains("/ready") {
        let uptime_secs = start_time.elapsed().as_secs();
        (
            "200 OK",
            "application/json",
            format!(r#"{{"status":"ready","uptime_seconds":{}}}"#, uptime_secs),
        )
    } else if first_line.contains("/metrics") {
        // Prometheus metrics endpoint
        let body = metrics.to_prometheus();
        ("200 OK", "text/plain; version=0.0.4; charset=utf-8", body)
    } else {
        // Default: gateway response
        metrics.request_start();
        let req_start = Instant::now();
        let body =
            r#"{"status":"ok","message":"VIL gateway running (no upstream)"}"#.to_string();
        metrics.request_end(req_start.elapsed().as_millis() as u64);
        ("200 OK", "application/json", body)
    }
}

pub fn run_with_mock(port: u16) -> Result<()> {
    // Start mock credit stream server on background thread
    let mock_port = MOCK_PORT;
    std::thread::spawn(move || {
        start_mock_credit_server(mock_port);
    });

    // Wait for mock to be ready
    std::thread::sleep(Duration::from_millis(200));

    println!(
        "  Mock credit stream: http://localhost:{}/api/v1/credits/stream",
        mock_port
    );
    println!("  VIL gateway:     http://localhost:{}/trigger", port);
    println!("  Press Ctrl+C to stop\n");

    // Start real VIL pipeline: port -> mock_port
    start_vil_pipeline(port, mock_port)
}

pub fn run_from_file(path: &str, port: u16) -> Result<()> {
    println!("Running pipeline from {} on port {}...", path, port);
    println!("(YAML pipeline support coming soon — running demo mode)\n");
    run_demo(port)
}

pub fn run_benchmark(requests: usize, concurrency: usize) -> Result<()> {
    let port: u16 = 3080;

    // Check if server is running
    match std::net::TcpStream::connect_timeout(
        &format!("127.0.0.1:{}", port).parse().unwrap(),
        Duration::from_secs(1),
    ) {
        Ok(_) => {}
        Err(_) => {
            println!("No server running on port {}.", port);
            println!("Start one first:\n");
            println!("  vil run --mock\n");
            println!("Then in another terminal:\n");
            println!("  vil bench -r {} -c {}\n", requests, concurrency);
            return Ok(());
        }
    }

    println!("Benchmarking http://localhost:{}/trigger", port);
    println!("  {} requests, {} concurrent\n", requests, concurrency);

    let url = format!("http://127.0.0.1:{}/trigger", port);
    let body = r#"{"request":"bench"}"#;

    let completed = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let errors = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let latencies = Arc::new(std::sync::Mutex::new(Vec::with_capacity(requests)));

    let start = Instant::now();

    // Distribute requests across worker threads
    let requests_per_worker = requests / concurrency;
    let remainder = requests % concurrency;

    let mut handles = Vec::new();
    for i in 0..concurrency {
        let n = if i < remainder {
            requests_per_worker + 1
        } else {
            requests_per_worker
        };
        let url = url.clone();
        let body = body.to_string();
        let completed = completed.clone();
        let errors = errors.clone();
        let latencies = latencies.clone();

        handles.push(std::thread::spawn(move || {
            for _ in 0..n {
                let req_start = Instant::now();
                match send_http_post(&url, &body) {
                    Ok(status) if status == 200 => {
                        let lat = req_start.elapsed();
                        completed.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                        latencies.lock().unwrap().push(lat);
                    }
                    _ => {
                        errors.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                    }
                }
            }
        }));
    }

    for h in handles {
        h.join().ok();
    }

    let total_time = start.elapsed();
    let ok = completed.load(std::sync::atomic::Ordering::Relaxed);
    let err = errors.load(std::sync::atomic::Ordering::Relaxed);

    let mut lats = latencies.lock().unwrap();
    lats.sort();

    let avg = if !lats.is_empty() {
        lats.iter().map(|d| d.as_secs_f64() * 1000.0).sum::<f64>() / lats.len() as f64
    } else {
        0.0
    };

    let p50 = percentile(&lats, 50.0);
    let p99 = percentile(&lats, 99.0);
    let fastest = lats
        .first()
        .map(|d| d.as_secs_f64() * 1000.0)
        .unwrap_or(0.0);
    let slowest = lats.last().map(|d| d.as_secs_f64() * 1000.0).unwrap_or(0.0);
    let rps = ok as f64 / total_time.as_secs_f64();

    println!("Summary:");
    println!(
        "  Success rate: {:.2}%",
        ok as f64 / (ok + err).max(1) as f64 * 100.0
    );
    println!(
        "  Total:        {:.4} ms",
        total_time.as_secs_f64() * 1000.0
    );
    println!("  Requests/sec: {:.2}", rps);
    println!("  Fastest:      {:.4} ms", fastest);
    println!("  Slowest:      {:.4} ms", slowest);
    println!("  Average:      {:.4} ms", avg);
    println!("  P50:          {:.4} ms", p50);
    println!("  P99:          {:.4} ms", p99);
    if err > 0 {
        println!("  Errors:       {}", err);
    }
    println!();
    println!("Status code distribution:");
    println!("  [200] {} responses", ok);
    if err > 0 {
        println!("  [err] {} responses", err);
    }

    Ok(())
}

fn percentile(sorted: &[Duration], pct: f64) -> f64 {
    if sorted.is_empty() {
        return 0.0;
    }
    let idx = ((pct / 100.0) * sorted.len() as f64) as usize;
    let idx = idx.min(sorted.len() - 1);
    sorted[idx].as_secs_f64() * 1000.0
}

fn send_http_post(url: &str, body: &str) -> Result<u16> {
    // Parse host:port from url
    let stripped = url.strip_prefix("http://").unwrap_or(url);
    let (host_port, path) = stripped.split_once('/').unwrap_or((stripped, ""));
    let path = format!("/{}", path);

    let mut stream = std::net::TcpStream::connect(host_port)?;
    stream.set_read_timeout(Some(Duration::from_secs(5)))?;

    let request = format!(
        "POST {} HTTP/1.1\r\nHost: {}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        path, host_port, body.len(), body
    );
    stream.write_all(request.as_bytes())?;
    stream.flush()?;

    let mut response = [0u8; 256];
    let n = stream.read(&mut response)?;
    let resp_str = String::from_utf8_lossy(&response[..n]);

    // Parse status code from "HTTP/1.1 200 OK"
    let status = resp_str
        .split_whitespace()
        .nth(1)
        .and_then(|s| s.parse::<u16>().ok())
        .unwrap_or(0);

    Ok(status)
}

/// Mock credit stream server — simulates Core Banking SSE endpoint.
/// Streams credit records in SSE format (event: records, event: complete).
fn start_mock_credit_server(port: u16) {
    let addr = format!("127.0.0.1:{}", port);
    let listener = match TcpListener::bind(&addr) {
        Ok(l) => l,
        Err(e) => {
            eprintln!("Failed to start mock server on {}: {}", addr, e);
            return;
        }
    };

    for stream in listener.incoming() {
        match stream {
            Ok(mut stream) => {
                let mut buffer = [0u8; 2048];
                let _ = stream.read(&mut buffer);

                let request = String::from_utf8_lossy(&buffer);
                let is_credit_stream = request.contains("/api/v1/credits/stream");

                if is_credit_stream {
                    // SSE streaming response (simulating Core Banking credit stream)
                    let body = "event: records\n\
                                data: [{\"id\":1,\"nik\":\"3201010101010001\",\"nama_lengkap\":\"Budi Santoso\",\"kolektabilitas\":1,\"jumlah_kredit\":50000000,\"saldo_outstanding\":35000000}]\n\n\
                                event: records\n\
                                data: [{\"id\":2,\"nik\":\"3201010101010002\",\"nama_lengkap\":\"Siti Rahayu\",\"kolektabilitas\":3,\"jumlah_kredit\":100000000,\"saldo_outstanding\":95000000}]\n\n\
                                event: records\n\
                                data: [{\"id\":3,\"nik\":\"3201010101010003\",\"nama_lengkap\":\"Ahmad Hidayat\",\"kolektabilitas\":5,\"jumlah_kredit\":200000000,\"saldo_outstanding\":200000000}]\n\n\
                                event: complete\n\
                                data: {\"total\":3,\"duration_ms\":150}\n\n";
                    let response = format!(
                        "HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nCache-Control: no-cache\r\nContent-Length: {}\r\n\r\n{}",
                        body.len(), body
                    );
                    let _ = stream.write_all(response.as_bytes());
                } else {
                    let body = r#"{"status":"ok","server":"vil-mock-credit-stream"}"#;
                    let response = format!(
                        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}",
                        body.len(), body
                    );
                    let _ = stream.write_all(response.as_bytes());
                }
            }
            Err(_) => {}
        }
    }
}

fn start_vil_pipeline(gateway_port: u16, upstream_port: u16) -> Result<()> {
    let upstream_url = format!(
        "http://127.0.0.1:{}/api/v1/credits/stream?count=10&batch_size=5&delay_ms=50",
        upstream_port
    );

    let world = Arc::new(
        vil_rt::VastarRuntimeWorld::new_shared()
            .map_err(|e| anyhow::anyhow!("Failed to initialize VIL SHM runtime: {}", e))?,
    );

    let sink_builder = vil_sdk::http::HttpSinkBuilder::new("WebhookTrigger")
        .port(gateway_port)
        .path("/trigger")
        .out_port("trigger_out")
        .in_port("data_in")
        .ctrl_in_port("ctrl_in");

    let source_builder = vil_sdk::http::HttpSourceBuilder::new("CreditStream")
        .url(&upstream_url)
        .format(vil_sdk::http::HttpFormat::SSE)
        .dialect(vil_sdk::http::SseSourceDialect::Standard)
        .in_port("trigger_in")
        .out_port("data_out")
        .ctrl_out_port("ctrl_out");

    let (_ir, (sink_handle, source_handle)) = vil_sdk::vil_workflow! {
        name: "VilCliPipeline",
        instances: [ sink_builder, source_builder ],
        routes: [
            sink_builder.trigger_out -> source_builder.trigger_in (LoanWrite),
            source_builder.data_out -> sink_builder.data_in (LoanWrite),
            source_builder.ctrl_out -> sink_builder.ctrl_in (Copy),
        ]
    };

    println!("Test with:");
    println!("  curl -N -X POST -H 'Content-Type: application/json' \\");
    println!(
        "    -d '{{\"request\": \"stream-credits\"}}' http://localhost:{}/trigger\n",
        gateway_port
    );
    println!("Load test:");
    println!("  vil bench -r 500 -c 50\n");

    let sink = vil_sdk::http::HttpSink::from_builder(sink_builder);
    let source = vil_sdk::http::HttpSource::from_builder(source_builder);

    let t1 = sink.run_worker::<vil_types::GenericToken>(world.clone(), sink_handle);
    let t2 = source.run_worker::<vil_types::GenericToken>(world.clone(), source_handle);

    t1.join().expect("Sink worker panicked");
    t2.join().expect("Source worker panicked");

    Ok(())
}
