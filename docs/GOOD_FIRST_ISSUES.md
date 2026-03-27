# Good First Issues for New Contributors

These issues are designed to help new contributors get familiar with the VIL codebase. Each issue is scoped to a single file or module and includes clear acceptance criteria.

---

## Documentation

### 1. Add doc examples to `vil_types` public items
**Labels**: `good first issue`, `documentation`
**File**: `crates/vil_types/src/lib.rs`
**Task**: Add `/// # Example` doc comments with runnable examples to all public structs and enums.
**Acceptance**: `cargo doc --no-deps` shows examples on all public items.

### 2. Improve error messages in `vil_validate`
**Labels**: `good first issue`, `enhancement`
**File**: `crates/vil_validate/src/lib.rs`
**Task**: Add human-readable error messages with fix suggestions to validation errors. Reference error codes from `vil_cli/src/errors.rs`.
**Acceptance**: Validation errors include `E-VIL-*` codes and suggested fixes.

### 3. Add README to each crate
**Labels**: `good first issue`, `documentation`
**Files**: `crates/*/README.md`
**Task**: Create a short README for each crate explaining its purpose, key types, and usage.
**Acceptance**: Each crate directory has a README.md that matches the crate's `description` in Cargo.toml.

---

## Examples

### 4. Create a "Hello Pipeline" minimal example
**Labels**: `good first issue`, `example`
**File**: `examples/002-hello-pipeline/`
**Task**: Create the simplest possible VIL pipeline — just two nodes exchanging a message. Use Layer 1 API (`vil_sdk::http_gateway()`).
**Acceptance**: `cargo run --example hello-pipeline` produces output.

### 5. Create a fan-out example
**Labels**: `good first issue`, `example`
**File**: `examples/003-fanout-demo/`
**Task**: Demonstrate one-to-many message broadcasting using VIL's Tri-Lane protocol.
**Acceptance**: Shows messages being sent to multiple consumers.

---

## Code

### 6. Add `Display` impl for `VilMetrics`
**Labels**: `good first issue`, `enhancement`
**File**: `crates/vil_obs/src/prometheus.rs`
**Task**: Implement `std::fmt::Display` for `VilMetrics` to show a human-readable summary (similar to `CounterSnapshot::Display`).
**Acceptance**: `println!("{}", metrics)` prints a readable summary.

### 7. Add `--format json` flag to `vil metrics`
**Labels**: `good first issue`, `enhancement`
**File**: `crates/vil_cli/src/main.rs`
**Task**: Add a `--format` flag to the `metrics` subcommand that outputs JSON instead of the default table format.
**Acceptance**: `vil metrics --format json` outputs valid JSON.

### 8. Add `vil version` subcommand
**Labels**: `good first issue`, `enhancement`
**File**: `crates/vil_cli/src/main.rs`
**Task**: Add a `version` subcommand that shows VIL version, Rust version, and platform info.
**Acceptance**: `vil version` shows version, rustc version, OS.

### 9. Add connection timeout to `HttpSourceBuilder`
**Labels**: `good first issue`, `enhancement`
**File**: `crates/vil_http/src/source.rs`
**Task**: Add a configurable connection timeout (default 30s) to `HttpSourceBuilder` so the source doesn't hang indefinitely when the upstream is down.
**Acceptance**: Builder has `.timeout(Duration)` method; connection fails fast when upstream is unreachable.

### 10. Add request counting to YAML pipeline runner
**Labels**: `good first issue`, `enhancement`
**File**: `crates/vil_cli/src/yaml_pipeline.rs`
**Task**: Track and log request count when running a YAML pipeline. Print a summary on Ctrl+C.
**Acceptance**: Running a YAML pipeline shows periodic request count updates.

---

## Testing

### 11. Add integration test for `vil new` templates
**Labels**: `good first issue`, `test`
**File**: `crates/vil_cli/tests/`
**Task**: Write a test that runs `vil new` for each template and verifies the generated project compiles with `cargo check`.
**Acceptance**: `cargo test -p vil_cli` passes with template generation tests.

### 12. Add benchmark for SHM allocator
**Labels**: `good first issue`, `performance`
**File**: `crates/vil_shm/benches/`
**Task**: Create a criterion benchmark measuring allocation/deallocation throughput of `ExchangeHeap`.
**Acceptance**: `cargo bench -p vil_shm` produces allocation throughput numbers.

---

## vil-server

### 13. Implement ConsulDiscovery adapter
**Labels**: `good first issue`, `vil-server`, `mesh`
**File**: `crates/vil_server_mesh/src/discovery.rs`
**Task**: Implement `ServiceDiscovery` trait for HashiCorp Consul. Use Consul HTTP API for service registration and health checking.
**Acceptance**: `ConsulDiscovery` resolves service endpoints from a running Consul agent.

### 14. Add request body size limit middleware
**Labels**: `good first issue`, `vil-server`, `security`
**File**: `crates/vil_server_core/src/`
**Task**: Create a Tower Layer that rejects requests with body larger than configurable limit. Return 413 Payload Too Large.
**Acceptance**: Requests exceeding limit are rejected with proper error response.

### 15. Implement `vil_db_mongodb` plugin
**Labels**: `good first issue`, `vil-server`, `database`
**File**: New crate `crates/vil_db_mongodb/`
**Task**: Implement `DbPool` trait from `vil_server_db` using the `mongodb` crate. Include connection pooling and health check.
**Acceptance**: MongoDB plugin connects, queries, and reports health via `/admin/plugins`.

### 16. Write benchmark: vil-server vs Actix-web
**Labels**: `good first issue`, `vil-server`, `performance`
**File**: `benchmarks/src/`
**Task**: Add an Actix-web benchmark binary alongside the existing Axum comparison. Compare GET hello, GET JSON, POST echo.
**Acceptance**: Benchmark report includes Actix-web numbers.

### 17. Add OpenTelemetry OTLP exporter
**Labels**: `good first issue`, `vil-server`, `observability`
**File**: `crates/vil_server_core/src/otel.rs`
**Task**: Export collected spans to an OTLP-compatible backend (Jaeger, Tempo) via HTTP. Use the existing `SpanCollector` as data source.
**Acceptance**: Spans appear in Jaeger when configured with OTLP endpoint.

### 18. Implement EtcdDiscovery adapter
**Labels**: `good first issue`, `vil-server`, `mesh`
**File**: `crates/vil_server_mesh/src/discovery.rs`
**Task**: Implement `ServiceDiscovery` trait for etcd. Use etcd HTTP API for key-value based service registration.
**Acceptance**: `EtcdDiscovery` resolves service endpoints from etcd.

---

**Note**: If you want to work on any of these, please comment on the GitHub issue first to avoid duplicate work.
