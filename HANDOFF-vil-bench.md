# HANDOFF: vil bench + Dashboard Quick Test

**Date:** 2026-03-30
**Status:** PLAN — ready for implementation
**Context:** Thread ini sudah handle E2E baseline, vil init flow, simulator refactor, website sync. Fitur `vil bench` + Dashboard Quick Test adalah next logical step.

---

## Problem

1. User harus install `hey` terpisah dan copy-paste command panjang
2. Tidak ada built-in benchmarking — user harus paham tools external
3. Dashboard hanya monitoring — tidak bisa test/bench langsung dari UI
4. Warmup step manual — seharusnya otomatis

## Solution: 2 Features

### Feature 1: `vil bench` CLI

Built-in benchmark tool di `vil_cli`. User cukup:

```bash
vil bench                              # auto-detect gateway + upstream
vil bench --target http://localhost:3080/api/gw/trigger
vil bench --upstream http://localhost:4545/v1/chat/completions
vil bench --concurrency 300 --duration 30s
vil bench --compare                    # upstream vs gateway side-by-side
```

#### Implementation Plan

1. **File:** `crates/vil_cli/src/bench.rs` (new module)
2. **Dependencies:** `reqwest` (sudah ada di vil_cli), `tokio`, `std::time`
3. **No external dependency** — built-in HTTP benchmark engine
4. **Auto-detect dari VASTAR_HOME:**
   - Scan `~/vastar/*/Cargo.toml` untuk port
   - Cek `app.vil.yaml` untuk upstream URL
   - Auto-warmup sebelum bench

#### CLI Spec

```
vil bench [OPTIONS]

Options:
  -t, --target <URL>        Gateway endpoint (auto-detect from app.vil.yaml)
  -u, --upstream <URL>       Upstream endpoint (auto-detect from app.vil.yaml)
  -c, --concurrency <N>     Concurrent connections [default: 200]
  -n, --requests <N>        Total requests [default: 3000]
  -z, --duration <DURATION>  Duration mode (e.g. 30s, 1m) — overrides -n
  -d, --data <JSON>          Request body [default: {"prompt":"bench"}]
  --compare                  Run upstream + gateway and show overhead
  --no-warmup                Skip warmup phase
  --json                     Output as JSON (for CI/CD)
```

#### Output Format

```
vil bench — AI Gateway Benchmark

  Warmup: 1000 requests (3s) ✓

  ┌─────────────────────────────────────────────────────────┐
  │ Upstream (http://localhost:4545)                         │
  │ Requests/sec: 6,200  |  p50: 41ms  |  p99: 53ms        │
  ├─────────────────────────────────────────────────────────┤
  │ Gateway  (http://localhost:3080/api/gw/trigger)         │
  │ Requests/sec: 5,900  |  p50: 42ms  |  p99: 47ms        │
  ├─────────────────────────────────────────────────────────┤
  │ Overhead: -4.8% throughput  |  +1ms p50  |  +0ms p99    │
  └─────────────────────────────────────────────────────────┘

  100% success  |  3000 requests  |  c300
```

#### Architecture

```
vil_cli/src/
  bench.rs          # Main bench module
    BenchConfig     # CLI args → config
    BenchRunner     # HTTP engine (tokio + reqwest)
    BenchResult     # Stats collection
    report()        # Pretty output
    compare()       # Side-by-side upstream vs gateway
```

#### HTTP Engine (built-in, no hey/oha)

```rust
// Simplified — spawn N tokio tasks, each sends requests in loop
async fn run_bench(config: &BenchConfig) -> BenchResult {
    let client = reqwest::Client::builder()
        .pool_max_idle_per_host(config.concurrency)
        .build()?;

    let semaphore = Arc::new(Semaphore::new(config.concurrency));
    let stats = Arc::new(StatsCollector::new());

    let tasks: Vec<_> = (0..config.total_requests)
        .map(|_| {
            let client = client.clone();
            let sem = semaphore.clone();
            let stats = stats.clone();
            tokio::spawn(async move {
                let _permit = sem.acquire().await;
                let start = Instant::now();
                let resp = client.post(&config.url)
                    .header("Content-Type", "application/json")
                    .body(config.body.clone())
                    .send().await;
                let elapsed = start.elapsed();
                stats.record(elapsed, resp.is_ok());
            })
        })
        .collect();

    join_all(tasks).await;
    stats.finalize()
}
```

### Feature 2: Dashboard Quick Test

Embed Postman-like UI di `/_vil/dashboard/` (existing observer dashboard).

#### UI Components

1. **Quick Test Tab** — manual request builder
   - Method selector (GET/POST/PUT/DELETE)
   - URL input (pre-filled dari service endpoints)
   - Headers editor
   - Body editor (JSON)
   - Send button → response viewer
   - Response time + status code

2. **Bench Tab** — visual benchmark
   - Concurrency slider (50-1000)
   - Duration/count toggle
   - Start/Stop button
   - **Live histogram** (updates every second)
   - **Live req/s counter**
   - **Upstream vs Gateway** comparison chart
   - Export results as JSON

#### Implementation Plan

1. **File:** `crates/vil_observer/src/dashboard_bench.html` (embedded HTML)
2. **API endpoints** di observer:
   - `POST /_vil/api/bench/start` — start benchmark
   - `GET /_vil/api/bench/status` — SSE stream of live stats
   - `POST /_vil/api/bench/stop` — stop benchmark
   - `POST /_vil/api/proxy` — proxy request for Quick Test
3. **Frontend:** vanilla JS (no framework) — keep it embedded + lightweight
4. **Backend:** tokio tasks untuk bench engine, SSE untuk live updates

#### Dashboard Layout

```
┌──────────────────────────────────────────────────────┐
│  VIL Observer Dashboard                              │
│  ┌──────┬──────────┬───────────┐                     │
│  │Metrics│Quick Test│ Benchmark │                     │
│  └──────┴──────────┴───────────┘                     │
│                                                      │
│  Quick Test:                                         │
│  ┌──────────────────────────────────────────────┐    │
│  │ POST ▼ │ /api/gw/trigger                     │    │
│  ├──────────────────────────────────────────────┤    │
│  │ Body:                                        │    │
│  │ { "prompt": "hello" }                        │    │
│  ├──────────────────────────────────────────────┤    │
│  │ [Send]                    200 OK  42ms        │    │
│  ├──────────────────────────────────────────────┤    │
│  │ Response:                                    │    │
│  │ { "content": "..." }                         │    │
│  └──────────────────────────────────────────────┘    │
│                                                      │
│  Benchmark:                                          │
│  ┌──────────────────────────────────────────────┐    │
│  │ Concurrency: [====200====]  Duration: 30s    │    │
│  │ [▶ Start]                                    │    │
│  │                                              │    │
│  │ Live: 5,832 req/s  |  p50: 42ms  |  p99: 48ms│   │
│  │ ████████████████████████████████ 100%          │   │
│  └──────────────────────────────────────────────┘    │
└──────────────────────────────────────────────────────┘
```

## Implementation Priority

1. **Phase 1:** `vil bench` CLI (1-2 days)
   - Basic HTTP engine
   - Auto-detect target/upstream
   - Compare mode
   - Pretty output

2. **Phase 2:** Dashboard Quick Test (1 day)
   - Proxy endpoint
   - Simple request builder UI
   - Response viewer

3. **Phase 3:** Dashboard Benchmark (2 days)
   - Bench engine in observer
   - SSE live stats stream
   - Live histogram UI
   - Upstream vs Gateway comparison

## Dependencies

- `vil_cli` — bench.rs module, clap subcommand
- `vil_observer` — Quick Test + Bench API endpoints
- `vil_server_core` — dashboard HTML update

## Testing

- `vil bench --target http://localhost:3080/api/gw/trigger -c 200 -n 1000`
- Compare output with `hey` results — should be within 5%
- Dashboard Quick Test: manual testing via browser
- Dashboard Bench: compare live stats with CLI results

## Rollout

1. Implement + test locally
2. Update `vil init` Next steps — replace hey with `vil bench`
3. Update website quickstart
4. Publish to crates.io
5. Remove hey dependency from Step 0

## Current State (for next thread)

- **vil_cli** v0.1.15 on crates.io
- **vil_server_core** v0.1.10 on crates.io
- **vil_server** v0.1.8 on crates.io
- **ai-endpoint-simulator** v0.3.3 on crates.io (zero Redis, embedded data)
- **vil_log** v0.1.3 — silent fallback (no spam)
- **Website** synced at vastar.id/products/vil
- **VASTAR_HOME** = ~/vastar/ workspace pattern
- **VilApp pattern** for ai-gateway template
- **ensure_port_free()** on VilApp + HttpSinkBuilder
- **hey** as current bench tool (pre-built binary)
- **oha** has TCP connection issue on user machine (not used)
