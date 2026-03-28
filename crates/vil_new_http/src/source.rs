// =============================================================================
// vil_new_http::source.rs — Thin HttpSource Node
// =============================================================================
// Demonstrates how source nodes are simplified by the new core primitives:
// 1. PortIR natively defines lane semantics (Trigger, Data, Control).
// 2. Control signals use standard `vil_types::ControlSignal` instead of
//    custom adapter-specific markers.
// 3. Clearer separation between HTTP pulling (physics) and token framing (protocol).
// =============================================================================

use futures_util::StreamExt;
use reqwest::Client;
use std::sync::Arc;
use tokio::runtime::Runtime;

use vil_ir::builder::WorkflowBuilder;
use vil_ir::core::{InterfaceIR, PortIR, ProcessIR, QueueIR};
use vil_rt::VastarRuntimeWorld;
use vil_types::{
    BoundaryKind, CleanupPolicy, ControlSignal, DeliveryGuarantee, ExecClass, GenericToken,
    LaneKind, ObservabilitySpec, PortDirection, PortSpec, Priority, ProcessSpec, QueueKind,
    ReactiveInterfaceKind, TransferMode,
};

use crate::HttpFormat;

/// SSE dialect preset for HttpSourceBuilder.
#[derive(Debug, Clone)]
pub enum SseSourceDialect {
    /// OpenAI / Mistral: done = `data: [DONE]`, tap = `choices[0].delta.content`
    OpenAi,
    /// Anthropic Claude: done = `event: message_stop`, tap = `delta.text`
    Anthropic,
    /// Ollama: done = `"done": true` in JSON, tap = `message.content`
    Ollama,
    /// Cohere: done = `event: message-end`, tap = `text`
    Cohere,
    /// Google Gemini: done = TCP EOF, tap = `candidates[0].content.parts[0].text`
    Gemini,
    /// W3C Standard: done = TCP EOF, no tap
    Standard,
    /// Custom configuration
    Custom {
        done_marker: Option<String>,
        done_event: Option<String>,
        done_json_field: Option<(String, serde_json::Value)>,
    },
}

pub struct HttpSourceBuilder {
    pub name: String,
    pub url: String,
    pub format: HttpFormat,
    pub method: String,
    pub json_body: Option<serde_json::Value>,
    pub json_tap: Option<String>,
    pub out_port_name: String,
    pub ctrl_out_port_name: Option<String>,
    pub in_port_name: Option<String>,
    pub capacity: usize,
    /// SSE done marker (e.g., "[DONE]" for OpenAI). None = EOF only.
    pub done_marker: Option<String>,
    /// SSE named event that signals done (e.g., "message_stop" for Anthropic).
    pub done_event: Option<String>,
    /// JSON field + value that signals done (e.g., ("done", true) for Ollama).
    pub done_json_field: Option<(String, serde_json::Value)>,
    /// Custom request headers (auth, content-type, etc.).
    pub headers: Vec<(String, String)>,
    /// User-defined transform function applied to each NDJSON line / SSE event.
    /// Return Some(bytes) to forward, None to filter out.
    pub transform_fn: Option<Arc<dyn Fn(&[u8]) -> Option<Vec<u8>> + Send + Sync>>,
}

impl HttpSourceBuilder {
    #[doc(alias = "vil_keep")]
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            url: String::new(),
            format: HttpFormat::Raw,
            method: "GET".into(),
            json_body: None,
            json_tap: None,
            out_port_name: "stream_out".into(),
            ctrl_out_port_name: Some("stream_ctrl".into()),
            in_port_name: None,
            capacity: 8192,
            done_marker: None,
            done_event: None,
            done_json_field: None,
            headers: Vec::new(),
            transform_fn: None,
        }
    }

    #[doc(alias = "vil_keep")]
    pub fn url(mut self, url: impl Into<String>) -> Self {
        self.url = url.into();
        self
    }
    #[doc(alias = "vil_keep")]
    pub fn format(mut self, format: HttpFormat) -> Self {
        self.format = format;
        self
    }
    #[doc(alias = "vil_keep")]
    pub fn post_json(mut self, body: serde_json::Value) -> Self {
        self.method = "POST".into();
        self.json_body = Some(body);
        self
    }
    #[doc(alias = "vil_keep")]
    pub fn json_tap(mut self, path: impl Into<String>) -> Self {
        self.json_tap = Some(path.into());
        self
    }
    #[doc(alias = "vil_keep")]
    pub fn out_port(mut self, name: impl Into<String>) -> Self {
        self.out_port_name = name.into();
        self
    }
    #[doc(alias = "vil_keep")]
    pub fn ctrl_out_port(mut self, name: impl Into<String>) -> Self {
        self.ctrl_out_port_name = Some(name.into());
        self
    }
    #[doc(alias = "vil_keep")]
    pub fn disable_ctrl_out_port(mut self) -> Self {
        self.ctrl_out_port_name = None;
        self
    }
    #[doc(alias = "vil_keep")]
    pub fn in_port(mut self, name: impl Into<String>) -> Self {
        self.in_port_name = Some(name.into());
        self
    }
    #[doc(alias = "vil_keep")]
    pub fn queue_capacity(mut self, capacity: usize) -> Self {
        self.capacity = capacity;
        self
    }

    /// Apply a dialect preset (sets done_marker, done_event, done_json_field, json_tap).
    #[doc(alias = "vil_keep")]
    pub fn dialect(mut self, d: SseSourceDialect) -> Self {
        match d {
            SseSourceDialect::OpenAi => {
                self.done_marker = Some("[DONE]".into());
                if self.json_tap.is_none() {
                    self.json_tap = Some("choices[0].delta.content".into());
                }
            }
            SseSourceDialect::Anthropic => {
                self.done_event = Some("message_stop".into());
                if self.json_tap.is_none() {
                    self.json_tap = Some("delta.text".into());
                }
            }
            SseSourceDialect::Ollama => {
                self.done_json_field = Some(("done".into(), serde_json::json!(true)));
                if self.json_tap.is_none() {
                    self.json_tap = Some("message.content".into());
                }
            }
            SseSourceDialect::Cohere => {
                self.done_marker = Some("[DONE]".into());
                self.done_event = Some("message-end".into());
                if self.json_tap.is_none() {
                    self.json_tap = Some("text".into());
                }
            }
            SseSourceDialect::Gemini => {
                if self.json_tap.is_none() {
                    self.json_tap = Some("candidates[0].content.parts[0].text".into());
                }
            }
            SseSourceDialect::Standard => {
                self.done_marker = None;
                self.done_event = None;
                self.done_json_field = None;
            }
            SseSourceDialect::Custom {
                done_marker,
                done_event,
                done_json_field,
            } => {
                self.done_marker = done_marker;
                self.done_event = done_event;
                self.done_json_field = done_json_field;
            }
        }
        self
    }

    /// Set data-line done marker (e.g., "[DONE]").
    #[doc(alias = "vil_keep")]
    pub fn done_marker(mut self, marker: impl Into<String>) -> Self {
        self.done_marker = Some(marker.into());
        self
    }

    /// Set named event type that signals done (e.g., "message_stop").
    #[doc(alias = "vil_keep")]
    pub fn done_event(mut self, event: impl Into<String>) -> Self {
        self.done_event = Some(event.into());
        self
    }

    /// Set JSON field + value that signals done (e.g., "done", true).
    #[doc(alias = "vil_keep")]
    pub fn done_json_field(mut self, field: impl Into<String>, value: serde_json::Value) -> Self {
        self.done_json_field = Some((field.into(), value));
        self
    }

    // ── Headers & Auth ──────────────────────────────────────────────

    /// Add a custom request header.
    #[doc(alias = "vil_keep")]
    pub fn header(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.headers.push((key.into(), value.into()));
        self
    }

    /// Set Bearer token auth (OpenAI, Cohere, Gemini).
    /// Adds `Authorization: Bearer <token>`.
    #[doc(alias = "vil_keep")]
    pub fn bearer_token(self, token: impl Into<String>) -> Self {
        self.header("Authorization", format!("Bearer {}", token.into()))
    }

    /// Set a transform function applied to each NDJSON line / SSE event.
    /// Return `Some(bytes)` to forward (transformed), `None` to filter out.
    ///
    /// # Example
    /// ```no_run
    /// builder.transform(|line: &[u8]| {
    ///     let record: serde_json::Value = serde_json::from_slice(line).ok()?;
    ///     if record["kolektabilitas"].as_u64()? >= 3 {
    ///         Some(line.to_vec()) // NPL — forward
    ///     } else {
    ///         None // healthy credit — filter out
    ///     }
    /// })
    /// ```
    pub fn transform<F>(mut self, f: F) -> Self
    where
        F: Fn(&[u8]) -> Option<Vec<u8>> + Send + Sync + 'static,
    {
        self.transform_fn = Some(Arc::new(f));
        self
    }

    /// Set Anthropic API key auth.
    /// Adds `x-api-key: <key>` + `anthropic-version: 2023-06-01`.
    #[doc(alias = "vil_keep")]
    pub fn anthropic_key(self, key: impl Into<String>) -> Self {
        self.header("x-api-key", key)
            .header("anthropic-version", "2023-06-01")
    }

    /// Set API key as URL query parameter (some Gemini endpoints).
    /// Appends `?key=<key>` to the URL.
    #[doc(alias = "vil_keep")]
    pub fn api_key_param(mut self, key: impl Into<String>) -> Self {
        let sep = if self.url.contains('?') { "&" } else { "?" };
        self.url = format!("{}{}key={}", self.url, sep, key.into());
        self
    }

    #[doc(alias = "vil_keep")]
    pub fn build_interface_ir(&self) -> InterfaceIR {
        let mut ports = std::collections::HashMap::new();
        ports.insert(
            self.out_port_name.clone(),
            PortIR {
                name: self.out_port_name.clone(),
                direction: PortDirection::Out,
                message_name: "StreamChunk".into(),
                queue_spec: QueueIR {
                    kind: QueueKind::Mpmc,
                    capacity: self.capacity,
                    backpressure: vil_types::BackpressurePolicy::Block,
                },
                timeout_ms: None,
                lane_kind: LaneKind::Data, // CORE PRIMITIVE: Formal Data Lane
            },
        );

        if let Some(ref ctrl_name) = self.ctrl_out_port_name {
            ports.insert(
                ctrl_name.clone(),
                PortIR {
                    name: ctrl_name.clone(),
                    direction: PortDirection::Out,
                    message_name: "ControlSignal".into(), // Switched from GenericToken to generic primitive
                    queue_spec: QueueIR {
                        kind: QueueKind::Mpmc,
                        capacity: self.capacity.max(256),
                        backpressure: vil_types::BackpressurePolicy::Block,
                    },
                    timeout_ms: None,
                    lane_kind: LaneKind::Control, // CORE PRIMITIVE: Formal Control Lane
                },
            );
        }

        if let Some(ref in_port) = self.in_port_name {
            ports.insert(
                in_port.clone(),
                PortIR {
                    name: in_port.clone(),
                    direction: PortDirection::In,
                    message_name: "GenericToken".into(),
                    queue_spec: QueueIR {
                        kind: QueueKind::Mpmc,
                        capacity: self.capacity,
                        backpressure: vil_types::BackpressurePolicy::Block,
                    },
                    timeout_ms: None,
                    lane_kind: LaneKind::Trigger, // CORE PRIMITIVE: Formal Trigger Lane
                },
            );
        }

        InterfaceIR {
            name: format!("{}Interface", self.name),
            ports,
            reactive_kind: ReactiveInterfaceKind::SessionReactive,
            host_affinity: None,
            trust_zone: None,
        }
    }

    #[doc(alias = "vil_keep")]
    pub fn build_process_ir(&self) -> ProcessIR {
        ProcessIR {
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

        if let Some(ref ctrl_name) = self.ctrl_out_port_name {
            ports.push(PortSpec {
                name: Box::leak(ctrl_name.clone().into_boxed_str()),
                direction: PortDirection::Out,
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

        if let Some(ref in_port) = self.in_port_name {
            ports.push(PortSpec {
                name: Box::leak(in_port.clone().into_boxed_str()),
                direction: PortDirection::In,
                queue: QueueKind::Mpmc,
                capacity: self.capacity,
                backpressure: vil_types::BackpressurePolicy::Block,
                transfer_mode: TransferMode::LoanWrite,
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

pub struct HttpSource {
    builder: HttpSourceBuilder,
}

impl HttpSource {
    #[doc(alias = "vil_keep")]
    pub fn from_builder(builder: HttpSourceBuilder) -> Self {
        Self { builder }
    }

    #[doc(alias = "vil_keep")]
    pub fn run_worker<T: crate::sink::StreamTokenLike>(
        self,
        world: Arc<VastarRuntimeWorld>,
        runtime_process: vil_rt::ProcessHandle,
    ) -> std::thread::JoinHandle<()> {
        let builder = Arc::new(self.builder);
        let world_clone = world.clone();
        let handle_clone = runtime_process.clone();
        let client = Arc::new(
            Client::builder()
                .tcp_nodelay(true)
                .pool_max_idle_per_host(1000)
                .pool_idle_timeout(Some(std::time::Duration::from_secs(90)))
                .build()
                .unwrap(),
        );

        std::thread::spawn(move || {
            let rt = Runtime::new().unwrap();
            let semaphore = Arc::new(tokio::sync::Semaphore::new(1024));

            rt.block_on(async move {
                if let Some(in_port_name) = &builder.in_port_name {
                    let in_port = handle_clone
                        .port_id(in_port_name)
                        .expect("Trigger port not found");
                    let mut spins = 0u64;
                    loop {
                        match world_clone.recv::<T>(in_port) {
                            Ok(loan) => {
                                spins = 0;
                                let session_id = loan.get().session_id();
                                let b = builder.clone();
                                let w = world_clone.clone();
                                let h = handle_clone.clone();
                                let c = client.clone();
                                let permit = semaphore.clone().acquire_owned().await.unwrap();
                                tokio::spawn(async move {
                                    let _permit = permit;
                                    execute_http_request::<T>(&b, w, &h, c, session_id).await;
                                });
                            }
                            Err(vil_rt::RtError::QueueEmpty(_)) => {
                                spins += 1;
                                if spins < 512 {
                                    std::hint::spin_loop();
                                } else if spins < 1024 {
                                    tokio::task::yield_now().await;
                                } else {
                                    tokio::time::sleep(std::time::Duration::from_micros(10)).await;
                                    spins = 0;
                                }
                            }
                            Err(e) => {
                                eprintln!("[HttpSource] Trigger error: {:?}", e);
                                break;
                            }
                        }
                    }
                } else {
                    execute_http_request::<T>(&builder, world_clone, &handle_clone, client, 0)
                        .await;
                }
            });
        })
    }
}

async fn execute_http_request<T: crate::sink::StreamTokenLike>(
    builder: &HttpSourceBuilder,
    world: Arc<VastarRuntimeWorld>,
    runtime_process: &vil_rt::ProcessHandle,
    client: Arc<Client>,
    session_id: u64,
) {
    let url = builder.url.clone();
    let format = builder.format;
    let method = builder.method.clone();
    let json_body = builder.json_body.clone();
    let json_tap = builder.json_tap.clone();
    let done_marker = builder.done_marker.clone();
    let done_event = builder.done_event.clone();
    let done_json_field = builder.done_json_field.clone();
    let transform_fn = builder.transform_fn.clone();
    let out_port = runtime_process
        .port_id(&builder.out_port_name)
        .expect("data out port not found");
    let ctrl_out_port = builder
        .ctrl_out_port_name
        .as_ref()
        .and_then(|name| runtime_process.port_id(name).ok());

    let mut req = match method.as_str() {
        "POST" => client.post(&url),
        _ => client.get(&url),
    };

    if let Some(body) = json_body {
        req = req.json(&body);
    }

    // Apply custom headers (auth tokens, content-type, etc.)
    for (key, value) in &builder.headers {
        req = req.header(key.as_str(), value.as_str());
    }

    let result = req.send().await;

    match result {
        Ok(response) => match format {
            HttpFormat::SSE => {
                // Zero-alloc SSE parse: raw bytes_stream, no eventsource String alloc.
                let mut stream = response.bytes_stream();
                let mut _current_event: Option<String> = None;

                'sse: while let Some(chunk_result) = stream.next().await {
                    match chunk_result {
                        Ok(chunk) => {
                            let text = std::str::from_utf8(&chunk).unwrap_or("");

                            for line in text.lines() {
                                // event: field
                                if let Some(evt) = line
                                    .strip_prefix("event: ")
                                    .or_else(|| line.strip_prefix("event:"))
                                {
                                    let evt = evt.trim();
                                    if let Some(ref de) = done_event {
                                        if evt == de.as_str() {
                                            break 'sse;
                                        }
                                    }
                                    _current_event = Some(evt.to_string());
                                    continue;
                                }

                                // data: field
                                if let Some(data_str) = line
                                    .strip_prefix("data: ")
                                    .or_else(|| line.strip_prefix("data:"))
                                {
                                    let data_str = data_str.trim();

                                    // Check done_marker
                                    if let Some(ref dm) = done_marker {
                                        if data_str == dm.as_str() {
                                            break 'sse;
                                        }
                                    }

                                    // Check done_json_field
                                    if let Some((ref field, ref expected)) = done_json_field {
                                        if let Ok(json) =
                                            vil_json::from_str::<serde_json::Value>(data_str)
                                        {
                                            if &json[field.as_str()] == expected {
                                                break 'sse;
                                            }
                                        }
                                    }

                                    // Zero-copy: slice directly from chunk bytes
                                    let data = bytes::Bytes::copy_from_slice(data_str.as_bytes());

                                    let final_data = if let Some(ref tap) = json_tap {
                                        apply_json_tap(data, tap)
                                    } else {
                                        data
                                    };

                                    if !final_data.is_empty() {
                                        let emit_data = if let Some(ref tf) = transform_fn {
                                            tf(&final_data).map(bytes::Bytes::from)
                                        } else {
                                            Some(final_data)
                                        };

                                        if let Some(data) = emit_data {
                                            let msg =
                                                T::from_sse_event_shm(data, session_id, &world);
                                            let _ = world.publish_value(
                                                runtime_process.id(),
                                                out_port,
                                                msg,
                                            );
                                        }
                                    }
                                }

                                // Empty line = event boundary
                                if line.is_empty() {
                                    _current_event = None;
                                }
                            }
                        }
                        Err(err) => {
                            eprintln!(
                                "[HttpSource] SSE stream error for session {}: {:?}",
                                session_id, err
                            );
                            if let Some(ctrl_port) = ctrl_out_port {
                                let _ = world.publish_value(
                                    runtime_process.id(),
                                    ctrl_port,
                                    ControlSignal::error(session_id, 500, "Stream Error"),
                                );
                            }
                            return;
                        }
                    }
                }
            }
            HttpFormat::NDJSON => {
                let mut stream = response.bytes_stream();
                let mut buffer = bytes::BytesMut::new();
                'ndjson: while let Some(chunk_result) = stream.next().await {
                    match chunk_result {
                        Ok(chunk) => {
                            buffer.extend_from_slice(&chunk);
                            while let Some(pos) = buffer.iter().position(|&b| b == b'\n') {
                                let line = buffer.split_to(pos + 1).freeze();

                                // Check done_json_field (e.g., "done": true for Ollama)
                                if let Some((ref field, ref expected)) = done_json_field {
                                    if let Ok(json) =
                                        vil_json::from_slice::<serde_json::Value>(&line)
                                    {
                                        if &json[field.as_str()] == expected {
                                            break 'ndjson;
                                        }
                                    }
                                }

                                let final_data = if let Some(ref tap) = json_tap {
                                    apply_json_tap(line, tap)
                                } else {
                                    line
                                };

                                if !final_data.is_empty() {
                                    // Apply user transform if set
                                    let emit_data = if let Some(ref tf) = transform_fn {
                                        tf(&final_data).map(|v| bytes::Bytes::from(v))
                                    } else {
                                        Some(final_data)
                                    };

                                    if let Some(data) = emit_data {
                                        let msg = T::from_ndjson_line_shm(data, session_id, &world);
                                        let _ = world.publish_value(
                                            runtime_process.id(),
                                            out_port,
                                            msg,
                                        );
                                    }
                                }
                            }
                        }
                        Err(err) => {
                            eprintln!(
                                "[HttpSource] NDJSON stream error for session {}: {:?}",
                                session_id, err
                            );
                            // CORE PRIMITIVE: emit Error signal immediately on stream error
                            if let Some(ctrl_port) = ctrl_out_port {
                                let _ = world.publish_value(
                                    runtime_process.id(),
                                    ctrl_port,
                                    ControlSignal::error(session_id, 500, "Stream Error"),
                                );
                            }
                            return; // early return
                        }
                    }
                }
            }
            HttpFormat::Raw => eprintln!("[HttpSource] Raw decoder not implemented yet"),
        },
        Err(err) => {
            eprintln!(
                "[HttpSource] Request error for session {}: {:?}",
                session_id, err
            );
            // CORE PRIMITIVE: emit Error signal immediately on request error
            if let Some(ctrl_port) = ctrl_out_port {
                let _ = world.publish_value(
                    runtime_process.id(),
                    ctrl_port,
                    ControlSignal::error(session_id, 503, "Request Failed"),
                );
            }
            return; // early return
        }
    }

    // CORE PRIMITIVE: publish explicit ControlSignal::Done instead of just marker or publish_control_done
    if let Some(ctrl_port) = ctrl_out_port {
        let _ = world.publish_value(
            runtime_process.id(),
            ctrl_port,
            ControlSignal::done(session_id),
        );
    } else {
        // Legacy fallback: keep in-band DONE if control lane is unavailable.
        let msg = T::done_marker(session_id);
        let _ = world.publish_value(runtime_process.id(), out_port, msg);
    }
}

pub trait WorkflowBuilderExt {
    fn add_http_source(self, builder: crate::source::HttpSourceBuilder) -> Self;
    fn add_http_sink(self, builder: crate::sink::HttpSinkBuilder) -> Self;
}

impl WorkflowBuilderExt for WorkflowBuilder {
    fn add_http_source(self, builder: crate::source::HttpSourceBuilder) -> Self {
        self.add_interface(builder.build_interface_ir())
            .add_process(builder.build_process_ir())
    }
    fn add_http_sink(self, builder: crate::sink::HttpSinkBuilder) -> Self {
        self.add_interface(builder.build_interface_ir())
            .add_process(builder.build_process_ir())
    }
}

pub trait FromStreamData {
    fn from_sse_event(data: bytes::Bytes, session_id: u64) -> Self;
    fn from_ndjson_line(data: bytes::Bytes, session_id: u64) -> Self;
    fn done_marker(session_id: u64) -> Self;

    /// Write payload to SHM and return a token with the offset.
    /// Default impl falls back to from_sse_event (no SHM — for GenericToken).
    /// ShmToken overrides this to write data to ExchangeHeap.
    fn from_sse_event_shm(data: bytes::Bytes, session_id: u64, world: &VastarRuntimeWorld) -> Self
    where
        Self: Sized,
    {
        // Default: ignore world, use heap path (GenericToken)
        let _ = world;
        Self::from_sse_event(data, session_id)
    }

    fn from_ndjson_line_shm(data: bytes::Bytes, session_id: u64, world: &VastarRuntimeWorld) -> Self
    where
        Self: Sized,
    {
        let _ = world;
        Self::from_ndjson_line(data, session_id)
    }
}

impl FromStreamData for GenericToken {
    fn from_sse_event(data: bytes::Bytes, session_id: u64) -> Self {
        Self {
            session_id,
            is_done: false,
            data: vil_types::VSlice::from_bytes(data),
        }
    }
    fn from_ndjson_line(data: bytes::Bytes, session_id: u64) -> Self {
        Self {
            session_id,
            is_done: false,
            data: vil_types::VSlice::from_bytes(data),
        }
    }
    fn done_marker(session_id: u64) -> Self {
        Self {
            session_id,
            is_done: true,
            data: vil_types::VSlice::from_vec(Vec::new()),
        }
    }
}

impl FromStreamData for vil_types::ShmToken {
    fn from_sse_event(_data: bytes::Bytes, session_id: u64) -> Self {
        // Fallback: no SHM context — return empty data token
        Self::data(session_id, 0, 0)
    }
    fn from_ndjson_line(_data: bytes::Bytes, session_id: u64) -> Self {
        Self::data(session_id, 0, 0)
    }
    fn done_marker(session_id: u64) -> Self {
        Self::done(session_id)
    }

    /// LOCK-FREE: Write payload to SHM via bump allocator.
    /// Single atomic fetch_add + memcpy — no mutex, no page scan.
    fn from_sse_event_shm(data: bytes::Bytes, session_id: u64, world: &VastarRuntimeWorld) -> Self {
        if let Some(heap) = world.exchange_heap() {
            if let Some(region) = world.data_region_id() {
                // Use bump allocator (lock-free) instead of paged allocator (mutex)
                if let Some((offset, len)) = heap.bump_alloc_and_write(region, &data) {
                    return Self::data(session_id, offset.as_u64(), len as u32);
                }
            }
        }
        Self::data(session_id, 0, 0)
    }

    fn from_ndjson_line_shm(
        data: bytes::Bytes,
        session_id: u64,
        world: &VastarRuntimeWorld,
    ) -> Self {
        Self::from_sse_event_shm(data, session_id, world)
    }
}

fn apply_json_tap(data: bytes::Bytes, path: &str) -> bytes::Bytes {
    if path == "choices[0].delta.content" {
        if let Some(pos) = find_subsequence(&data, b"\"content\":\"") {
            let start = pos + 11;
            if let Some(end) = data[start..].iter().position(|&b| b == b'\"') {
                let content_end = start + end;
                if !data[start..content_end].contains(&b'\\') {
                    return data.slice(start..content_end);
                }
            }
        }
    }
    if let Ok(val) = vil_json::from_slice::<serde_json::Value>(&data) {
        let mut current = &val;
        for part in path.split('.') {
            if let Some(idx_start) = part.find('[') {
                let key = &part[..idx_start];
                current = &current[key];
                if let Some(idx_end) = part.find(']') {
                    if let Ok(idx) = part[idx_start + 1..idx_end].parse::<usize>() {
                        current = &current[idx];
                    }
                }
            } else {
                current = &current[part];
            }
        }
        match current {
            serde_json::Value::String(s) => {
                let s_bytes = s.as_bytes();
                if !s.contains('\\') {
                    if let Some(pos) = find_subsequence(&data, s_bytes) {
                        return data.slice(pos..pos + s_bytes.len());
                    }
                }
                bytes::Bytes::copy_from_slice(s_bytes)
            }
            serde_json::Value::Null => bytes::Bytes::new(),
            _ => bytes::Bytes::copy_from_slice(current.to_string().as_bytes()),
        }
    } else {
        bytes::Bytes::new()
    }
}

fn find_subsequence(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack
        .windows(needle.len())
        .position(|window| window == needle)
}
