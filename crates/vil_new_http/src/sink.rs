// =============================================================================
// vil_new_http::sink.rs — Thin Webhook Trigger
// =============================================================================
// Demonstrates how thin an adapter becomes when using the core session fabric:
// 1. No custom DashMaps for session state.
// 2. No custom pending buffers or TTL logic.
// 3. Dispatchers reduced to simple registry method calls.
// 4. Clean separation of HTTP/Axum (protocol) vs Registry (physics).
// =============================================================================

use std::net::SocketAddr;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;

// ── Global inbound request counters (for observer sidecar) ──────────────
static INBOUND_REQUESTS: AtomicU64 = AtomicU64::new(0);
static INBOUND_COMPLETED: AtomicU64 = AtomicU64::new(0);
static INBOUND_ERRORS: AtomicU64 = AtomicU64::new(0);
static INBOUND_LATENCY_SUM_NS: AtomicU64 = AtomicU64::new(0);
static INBOUND_MIN_NS: AtomicU64 = AtomicU64::new(u64::MAX);
static INBOUND_MAX_NS: AtomicU64 = AtomicU64::new(0);

/// Latency histogram — same bucket boundaries as obs_middleware.
const LATENCY_BUCKETS: [u64; 40] = [
    10, 25, 50, 75, 100, 150, 200, 300, 500, 750, 1_000, 1_500, 2_000, 2_500, 3_000, 4_000, 5_000,
    6_000, 7_500, 10_000, 12_500, 15_000, 20_000, 25_000, 30_000, 40_000, 50_000, 60_000, 75_000,
    100_000, 125_000, 150_000, 200_000, 300_000, 500_000, 750_000, 1_000_000, 1_500_000, 2_000_000,
    5_000_000,
];
static INBOUND_BUCKETS: [AtomicU64; 41] = {
    // const init workaround
    const ZERO: AtomicU64 = AtomicU64::new(0);
    [ZERO; 41]
};

fn record_inbound_latency(duration_ns: u64) {
    INBOUND_COMPLETED.fetch_add(1, Ordering::Relaxed);
    INBOUND_LATENCY_SUM_NS.fetch_add(duration_ns, Ordering::Relaxed);

    // min (CAS)
    let mut cur = INBOUND_MIN_NS.load(Ordering::Relaxed);
    while duration_ns < cur {
        match INBOUND_MIN_NS.compare_exchange_weak(
            cur,
            duration_ns,
            Ordering::Relaxed,
            Ordering::Relaxed,
        ) {
            Ok(_) => break,
            Err(c) => cur = c,
        }
    }
    // max (CAS)
    let mut cur = INBOUND_MAX_NS.load(Ordering::Relaxed);
    while duration_ns > cur {
        match INBOUND_MAX_NS.compare_exchange_weak(
            cur,
            duration_ns,
            Ordering::Relaxed,
            Ordering::Relaxed,
        ) {
            Ok(_) => break,
            Err(c) => cur = c,
        }
    }
    // bucket
    let idx = LATENCY_BUCKETS
        .iter()
        .position(|&b| duration_ns <= b)
        .unwrap_or(LATENCY_BUCKETS.len());
    INBOUND_BUCKETS[idx].fetch_add(1, Ordering::Relaxed);
}

fn percentile_from_buckets(pct: f64) -> u64 {
    let counts: Vec<u64> = INBOUND_BUCKETS
        .iter()
        .map(|b| b.load(Ordering::Relaxed))
        .collect();
    let total: u64 = counts.iter().sum();
    if total == 0 {
        return 0;
    }
    let target = (total as f64 * pct).ceil() as u64;
    let mut cum = 0u64;
    for (i, &c) in counts.iter().enumerate() {
        cum += c;
        if cum >= target {
            return if i < LATENCY_BUCKETS.len() {
                LATENCY_BUCKETS[i]
            } else {
                LATENCY_BUCKETS[LATENCY_BUCKETS.len() - 1] * 2
            };
        }
    }
    LATENCY_BUCKETS[LATENCY_BUCKETS.len() - 1]
}

/// Get inbound HTTP request counters for observer sidecar.
pub fn inbound_snapshot() -> InboundSnapshot {
    let reqs = INBOUND_REQUESTS.load(Ordering::Relaxed);
    let completed = INBOUND_COMPLETED.load(Ordering::Relaxed);
    let errs = INBOUND_ERRORS.load(Ordering::Relaxed);
    let sum = INBOUND_LATENCY_SUM_NS.load(Ordering::Relaxed);
    let min = INBOUND_MIN_NS.load(Ordering::Relaxed);
    let max = INBOUND_MAX_NS.load(Ordering::Relaxed);
    InboundSnapshot {
        requests: reqs,
        completed,
        in_flight: reqs.saturating_sub(completed),
        errors: errs,
        avg_latency_ns: if completed > 0 { sum / completed } else { 0 },
        min_latency_ns: if min == u64::MAX { 0 } else { min },
        max_latency_ns: max,
        p95_ns: percentile_from_buckets(0.95),
        p99_ns: percentile_from_buckets(0.99),
        p999_ns: percentile_from_buckets(0.999),
    }
}

#[derive(serde::Serialize)]
pub struct InboundSnapshot {
    pub requests: u64,
    pub completed: u64,
    pub in_flight: u64,
    pub errors: u64,
    pub avg_latency_ns: u64,
    pub min_latency_ns: u64,
    pub max_latency_ns: u64,
    pub p95_ns: u64,
    pub p99_ns: u64,
    pub p999_ns: u64,
}

use axum::{
    body::Body,
    extract::State,
    response::{IntoResponse, Response},
    routing::post,
    Router,
};
use tokio::runtime::Runtime;

use crate::source::FromStreamData;
use vil_rt::session::{SessionConfig, SessionRegistry};
use vil_rt::world::SampleGuard;
use vil_rt::VastarRuntimeWorld;
use vil_types::{
    BoundaryKind, CleanupPolicy, ControlSignal, DeliveryGuarantee, ExecClass, GenericToken,
    LaneKind, ObservabilitySpec, PortDirection, PortSpec, Priority, ProcessSpec, QueueKind,
    ReactiveInterfaceKind, TransferMode,
};

pub trait StreamTokenLike:
    vil_types::MessageContract + FromStreamData + Send + Sync + 'static
{
    fn session_id(&self) -> u64;
    fn is_done(&self) -> bool;
    fn data_slice(&self) -> &vil_types::VSlice<u8>;

    /// Resolve payload bytes, optionally reading from SHM.
    /// Default: reads from data_slice() (GenericToken path).
    /// ShmToken overrides to read from ExchangeHeap at data_offset.
    fn resolve_payload(&self, world: &VastarRuntimeWorld) -> Option<bytes::Bytes> {
        let _ = world;
        let vslice = self.data_slice();
        let data = vslice.as_slice();
        if data.is_empty() {
            return None;
        }
        let offset = if data.starts_with(b"data: ") {
            6
        } else if data.starts_with(b"data:") {
            5
        } else {
            0
        };
        if offset > 0 {
            Some(vslice.slice_bytes(offset..data.len()).to_bytes())
        } else {
            Some(vslice.to_bytes())
        }
    }
}

impl StreamTokenLike for GenericToken {
    fn session_id(&self) -> u64 {
        self.session_id
    }
    fn is_done(&self) -> bool {
        self.is_done
    }
    fn data_slice(&self) -> &vil_types::VSlice<u8> {
        &self.data
    }
}

// ShmToken: data lives in SHM at offset. data_slice() returns empty because
// actual payload reading happens via world.exchange_heap() at the dispatch site.
// This enables ShmToken to participate in the StreamTokenLike pipeline without
// needing VSlice (which would defeat the zero-copy purpose).

fn empty_vslice() -> &'static vil_types::VSlice<u8> {
    static EMPTY: std::sync::OnceLock<vil_types::VSlice<u8>> = std::sync::OnceLock::new();
    EMPTY.get_or_init(|| vil_types::VSlice::from_vec(Vec::new()))
}

impl StreamTokenLike for vil_types::ShmToken {
    fn session_id(&self) -> u64 {
        self.session_id
    }
    fn is_done(&self) -> bool {
        self.status == 1
    }
    fn data_slice(&self) -> &vil_types::VSlice<u8> {
        empty_vslice()
    }

    /// LOCK-FREE READ: Read payload via bump region (direct pointer, no mutex).
    fn resolve_payload(&self, world: &VastarRuntimeWorld) -> Option<bytes::Bytes> {
        if self.is_done() || self.is_error() || self.data_len == 0 {
            return None;
        }
        if let (Some(heap), Some(region)) = (world.exchange_heap(), world.data_region_id()) {
            let offset = vil_rt::vil_shm::Offset::new(self.data_offset);
            // Try lock-free bump read first
            if let Some(bump) = heap.bump_region(region) {
                let data = unsafe { bump.read(offset, self.data_len as usize) };
                return Some(bytes::Bytes::copy_from_slice(data));
            }
            // Fallback to mutex-based read
            if let Some(raw) = heap.read_bytes(region, offset, self.data_len as usize) {
                return Some(bytes::Bytes::from(raw));
            }
        }
        None
    }
}

pub struct HttpSinkBuilder {
    pub name: String,
    pub port: u16,
    pub path: String,
    pub out_port_name: String,
    pub in_port_name: Option<String>,
    pub ctrl_in_port_name: Option<String>,
    pub capacity: usize,
}

impl HttpSinkBuilder {
    #[doc(alias = "vil_keep")]
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            port: 8080,
            path: "/trigger".into(),
            out_port_name: "webhook_out".into(),
            in_port_name: None,
            ctrl_in_port_name: Some("webhook_ctrl".into()),
            capacity: 32768,
        }
    }

    #[doc(alias = "vil_keep")]
    pub fn port(mut self, port: u16) -> Self {
        self.port = port;
        self
    }

    /// Bind to `PORT` env var when set, otherwise fall back to `default`.
    ///
    /// Use on the *primary* externally-exposed sink so CI/bench drivers can
    /// relocate the listener without patching source. Leave secondary sinks on
    /// plain `.port(...)` so they don't collide with the primary.
    #[doc(alias = "vil_keep")]
    pub fn env_port(self, default: u16) -> Self {
        let port = std::env::var("PORT")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(default);
        self.port(port)
    }

    /// Ensure the port is free before binding — kills any stale process using it.
    ///
    /// ```ignore
    /// HttpSinkBuilder::new("webhook")
    ///     .port(3080)
    ///     .ensure_port_free()
    /// ```
    #[doc(alias = "vil_keep")]
    pub fn ensure_port_free(self) -> Self {
        let port = self.port;
        if let Ok(listener) = std::net::TcpListener::bind(("0.0.0.0", port)) {
            drop(listener); // port is free
        } else {
            eprintln!("Port {} in use — releasing...", port);
            #[cfg(unix)]
            {
                let _ = std::process::Command::new("sh")
                    .args(["-c", &format!("kill $(lsof -ti:{}) 2>/dev/null", port)])
                    .output();
                std::thread::sleep(std::time::Duration::from_millis(500));
            }
            #[cfg(not(unix))]
            {
                eprintln!("  Please close the process on port {} manually.", port);
            }
        }
        self
    }

    #[doc(alias = "vil_keep")]
    pub fn path(mut self, path: impl Into<String>) -> Self {
        self.path = path.into();
        self
    }
    #[doc(alias = "vil_keep")]
    pub fn out_port(mut self, name: impl Into<String>) -> Self {
        self.out_port_name = name.into();
        self
    }
    #[doc(alias = "vil_keep")]
    pub fn in_port(mut self, name: impl Into<String>) -> Self {
        self.in_port_name = Some(name.into());
        self
    }
    #[doc(alias = "vil_keep")]
    pub fn ctrl_in_port(mut self, name: impl Into<String>) -> Self {
        self.ctrl_in_port_name = Some(name.into());
        self
    }
    #[doc(alias = "vil_keep")]
    pub fn disable_ctrl_in_port(mut self) -> Self {
        self.ctrl_in_port_name = None;
        self
    }
    #[doc(alias = "vil_keep")]
    pub fn queue_capacity(mut self, capacity: usize) -> Self {
        self.capacity = capacity;
        self
    }

    #[doc(alias = "vil_keep")]
    pub fn build_interface_ir(&self) -> vil_ir::core::InterfaceIR {
        let mut ports = std::collections::HashMap::new();
        ports.insert(
            self.out_port_name.clone(),
            vil_ir::core::PortIR {
                name: self.out_port_name.clone(),
                direction: PortDirection::Out,
                message_name: "GenericToken".into(),
                queue_spec: vil_ir::core::QueueIR {
                    kind: QueueKind::Mpmc,
                    capacity: self.capacity,
                    backpressure: vil_types::BackpressurePolicy::Block,
                },
                timeout_ms: None,
                lane_kind: LaneKind::Trigger,
            },
        );

        if let Some(ref in_port) = self.in_port_name {
            ports.insert(
                in_port.clone(),
                vil_ir::core::PortIR {
                    name: in_port.clone(),
                    direction: PortDirection::In,
                    message_name: "GenericToken".into(),
                    queue_spec: vil_ir::core::QueueIR {
                        kind: QueueKind::Mpmc,
                        capacity: self.capacity,
                        backpressure: vil_types::BackpressurePolicy::Block,
                    },
                    timeout_ms: None,
                    lane_kind: LaneKind::Data,
                },
            );
        }

        if let Some(ref ctrl_port) = self.ctrl_in_port_name {
            ports.insert(
                ctrl_port.clone(),
                vil_ir::core::PortIR {
                    name: ctrl_port.clone(),
                    direction: PortDirection::In,
                    message_name: "ControlSignal".into(),
                    queue_spec: vil_ir::core::QueueIR {
                        kind: QueueKind::Mpmc,
                        capacity: self.capacity.max(256),
                        backpressure: vil_types::BackpressurePolicy::Block,
                    },
                    timeout_ms: None,
                    lane_kind: LaneKind::Control,
                },
            );
        }

        vil_ir::core::InterfaceIR {
            name: format!("{}Interface", self.name),
            ports,
            reactive_kind: ReactiveInterfaceKind::Normal,
            host_affinity: None,
            trust_zone: None,
        }
    }

    #[doc(alias = "vil_keep")]
    pub fn build_process_ir(&self) -> vil_ir::core::ProcessIR {
        vil_ir::core::ProcessIR {
            name: self.name.clone(),
            interface_name: format!("{}Interface", self.name),
            exec_class: ExecClass::Thread,
            cleanup_policy: CleanupPolicy::ReclaimOrphans,
            priority: Priority::Normal,
            host_affinity: None,
            trust_zone: None,
            obs: Default::default(),
        }
    }

    #[doc(alias = "vil_keep")]
    pub fn build_spec(&self) -> ProcessSpec {
        let mut ports = vec![PortSpec {
            name: Box::leak(self.out_port_name.clone().into_boxed_str()),
            direction: PortDirection::Out,
            queue: QueueKind::Mpmc,
            capacity: self.capacity,
            backpressure: vil_types::BackpressurePolicy::Block,
            transfer_mode: TransferMode::LoanWrite,
            boundary: BoundaryKind::InterThreadLocal,
            timeout_ms: None,
            priority: Priority::Normal,
            delivery: DeliveryGuarantee::BestEffort,
            observability: ObservabilitySpec::default(),
        }];

        if let Some(ref in_port) = self.in_port_name {
            ports.push(PortSpec {
                name: Box::leak(in_port.clone().into_boxed_str()),
                direction: PortDirection::In,
                queue: QueueKind::Mpmc,
                capacity: self.capacity,
                backpressure: vil_types::BackpressurePolicy::Block,
                transfer_mode: TransferMode::LoanRead,
                boundary: BoundaryKind::InterThreadLocal,
                timeout_ms: None,
                priority: Priority::Normal,
                delivery: DeliveryGuarantee::BestEffort,
                observability: ObservabilitySpec::default(),
            });
        }

        if let Some(ref ctrl_in) = self.ctrl_in_port_name {
            ports.push(PortSpec {
                name: Box::leak(ctrl_in.clone().into_boxed_str()),
                direction: PortDirection::In,
                queue: QueueKind::Mpmc,
                capacity: self.capacity.max(256),
                backpressure: vil_types::BackpressurePolicy::Block,
                transfer_mode: TransferMode::Copy,
                boundary: BoundaryKind::InterThreadLocal,
                timeout_ms: None,
                priority: Priority::Normal,
                delivery: DeliveryGuarantee::BestEffort,
                observability: ObservabilitySpec::default(),
            });
        }

        ProcessSpec {
            id: Box::leak(self.name.to_lowercase().into_boxed_str()),
            name: Box::leak(self.name.clone().into_boxed_str()),
            exec: ExecClass::Thread,
            cleanup: CleanupPolicy::ReclaimOrphans,
            ports: Box::leak(ports.into_boxed_slice()),
            observability: ObservabilitySpec::default(),
        }
    }
}

pub struct HttpSink {
    builder: HttpSinkBuilder,
}

impl HttpSink {
    #[doc(alias = "vil_keep")]
    pub fn from_builder(builder: HttpSinkBuilder) -> Self {
        Self { builder }
    }

    #[doc(alias = "vil_keep")]
    pub fn run_worker<T: StreamTokenLike>(
        self,
        world: Arc<VastarRuntimeWorld>,
        runtime_process: vil_rt::ProcessHandle,
    ) -> std::thread::JoinHandle<()> {
        // PORT env overrides hardcoded port — enables bench/test port management
        let port = std::env::var("PORT")
            .ok()
            .and_then(|s| s.parse::<u16>().ok())
            .unwrap_or(self.builder.port);
        let path = self.builder.path.clone();
        let out_port = runtime_process
            .port_id(&self.builder.out_port_name)
            .expect("Out Port not found");
        let data_in_port = self.builder.in_port_name.as_ref().map(|name| {
            runtime_process
                .port_id(name)
                .expect("Data in-port not found")
        });
        let ctrl_in_port = self.builder.ctrl_in_port_name.as_ref().map(|name| {
            runtime_process
                .port_id(name)
                .expect("Control in-port not found")
        });

        // CORE PRIMITIVE: Core session fabric replaces custom DashMaps
        let registry = Arc::new(SessionRegistry::<SampleGuard<T>>::with_config(
            SessionConfig::default(),
        ));

        // Combined dispatch: single thread polls both data + ctrl queues.
        // Eliminates 1 thread wake + 1 context switch per message vs separate threads.
        if data_in_port.is_some() || ctrl_in_port.is_some() {
            let registry_clone = registry.clone();
            let world_clone = world.clone();
            std::thread::spawn(move || {
                dispatch_combined_loop::<T>(world_clone, data_in_port, ctrl_in_port, registry_clone)
            });
        }

        std::thread::spawn(move || {
            let rt = Runtime::new().unwrap();
            rt.block_on(async move {
                let state = AppState {
                    world,
                    process_id: runtime_process.id(),
                    out_port,
                    registry,
                    next_session_id: Arc::new(AtomicU64::new(1)),
                };

                let app = Router::new()
                    .route(&path, post(handle_webhook::<T>))
                    .with_state(Arc::new(state));

                let addr = SocketAddr::from(([0, 0, 0, 0], port));
                let listener = match tokio::net::TcpListener::bind(addr).await {
                    Ok(l) => l,
                    Err(e) => {
                        eprintln!(
                            "❌ [HttpSink] Failed to bind to {} : {}. Is another process running?",
                            addr, e
                        );
                        return;
                    }
                };

                if let Err(e) = axum::serve(listener, app).await {
                    eprintln!("[HttpSink] Server Error: {}", e);
                }
            });
        })
    }
}

struct AppState<T: StreamTokenLike> {
    world: Arc<VastarRuntimeWorld>,
    process_id: vil_types::ProcessId,
    out_port: vil_types::PortId,
    registry: Arc<SessionRegistry<SampleGuard<T>>>,
    next_session_id: Arc<AtomicU64>,
}

/// Combined data + ctrl dispatch in single thread.
/// Polls both queues in tight loop — saves 1 thread + 1 wake per message.
fn dispatch_combined_loop<T: StreamTokenLike>(
    world: Arc<VastarRuntimeWorld>,
    data_port: Option<vil_types::PortId>,
    ctrl_port: Option<vil_types::PortId>,
    registry: Arc<SessionRegistry<SampleGuard<T>>>,
) {
    let mut spins = 0u64;
    loop {
        let mut did_work = false;

        // Poll data queue
        if let Some(dp) = data_port {
            match world.recv::<T>(dp) {
                Ok(msg) => {
                    did_work = true;
                    let session_id = msg.session_id();
                    if msg.is_done() {
                        registry.deliver_control(session_id, ControlSignal::done(session_id));
                    } else {
                        registry.deliver_data(session_id, msg);
                    }
                }
                Err(vil_rt::RtError::QueueEmpty(_)) => {}
                Err(e) => {
                    eprintln!("[HttpSink] Data dispatch error: {:?}", e);
                }
            }
        }

        // Poll ctrl queue
        if let Some(cp) = ctrl_port {
            match world.recv::<ControlSignal>(cp) {
                Ok(ctrl_msg) => {
                    did_work = true;
                    let session_id = ctrl_msg.get().session_id();
                    registry.deliver_control(session_id, ctrl_msg.get().clone());
                }
                Err(vil_rt::RtError::QueueEmpty(_)) => {}
                Err(e) => {
                    eprintln!("[HttpSink] Ctrl dispatch error: {:?}", e);
                }
            }
        }

        if did_work {
            spins = 0;
        } else {
            spins += 1;
            if spins < 1024 {
                std::hint::spin_loop();
            } else if spins < 2048 {
                std::thread::yield_now();
            } else {
                std::thread::sleep(Duration::from_micros(10));
                spins = 0;
            }
        }
    }
}

fn normalize_payload<T: StreamTokenLike>(
    token_guard: &SampleGuard<T>,
    world: &VastarRuntimeWorld,
) -> Option<bytes::Bytes> {
    if token_guard.is_done() {
        return None;
    }
    token_guard.get().resolve_payload(world)
}

async fn handle_webhook<T: StreamTokenLike>(
    State(state): State<Arc<AppState<T>>>,
    payload: bytes::Bytes,
) -> Response {
    let _req_start = std::time::Instant::now();
    INBOUND_REQUESTS.fetch_add(1, Ordering::Relaxed);

    let session_id = state.next_session_id.fetch_add(1, Ordering::Relaxed);

    // CORE PRIMITIVE: Delegate session setup and pending flush to Registry
    let (mut data_rx, mut ctrl_rx) = state.registry.register(session_id);

    struct SessionGuard<T: StreamTokenLike> {
        id: u64,
        registry: Arc<SessionRegistry<SampleGuard<T>>>,
        start: std::time::Instant,
    }
    impl<T: StreamTokenLike> Drop for SessionGuard<T> {
        fn drop(&mut self) {
            self.registry.cleanup(self.id);
            let dur = self.start.elapsed().as_nanos() as u64;
            record_inbound_latency(dur);
        }
    }

    let guard = SessionGuard {
        id: session_id,
        registry: state.registry.clone(),
        start: _req_start,
    };

    let trigger_msg = T::from_ndjson_line_shm(payload, session_id, &state.world);
    if state
        .world
        .publish_value(state.process_id, state.out_port, trigger_msg)
        .is_err()
    {
        return axum::http::StatusCode::SERVICE_UNAVAILABLE.into_response();
    }

    let stream = async_stream::stream! {
        let _keep_alive = guard;
        let mut done_seen = false;

        loop {
            tokio::select! {
                biased;
                ctrl = ctrl_rx.recv() => {
                    match ctrl {
                        Some(ControlSignal::Done { .. }) => { done_seen = true; break; }
                        Some(ControlSignal::Error { reason, .. }) => {
                            yield Ok::<_, std::convert::Infallible>(bytes::Bytes::from(format!("error:{}", reason)));
                            done_seen = true;
                            break;
                        }
                        Some(ControlSignal::Abort { .. }) => { done_seen = true; break; }
                        None => break,
                    }
                }
                data = data_rx.recv() => {
                    match data {
                        Some(token_guard) => {
                            if let Some(chunk) = normalize_payload(&token_guard, &state.world) {
                                yield Ok::<_, std::convert::Infallible>(chunk);
                            }
                        }
                        None => break,
                    }
                }
                _ = tokio::time::sleep(state.registry.config().session_timeout) => { break; }
            }
        }

        if done_seen {
            while let Ok(token_guard) = data_rx.try_recv() {
                if let Some(chunk) = normalize_payload(&token_guard, &state.world) {
                    yield Ok::<_, std::convert::Infallible>(chunk);
                }
            }
        }
    };

    Response::builder()
        .header("content-type", "text/event-stream")
        .header("cache-control", "no-cache")
        .header("connection", "keep-alive")
        .body(Body::from_stream(stream))
        .unwrap()
        .into_response()
}
