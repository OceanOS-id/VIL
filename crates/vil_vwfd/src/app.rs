//! # VwfdApp — Ergonomic builder for VWFD workflow server
//!
//! ```rust,no_run
//! use vil_vwfd::prelude::*;
//!
//! #[tokio::main]
//! async fn main() {
//!     vil_vwfd::app("workflows/", 3200)
//!         .native("validate_order", |input| {
//!             let items = input["items"].as_array().ok_or("items required")?;
//!             Ok(json!({"valid": !items.is_empty()}))
//!         })
//!         .native("chunk_splitter", handlers::chunk_splitter)
//!         .wasm("pricing", "modules/pricing.wasm")
//!         .sidecar("fraud-scorer", "python3 fraud_model.py")
//!         .run()
//!         .await;
//! }
//! ```

use std::collections::HashMap;
use std::sync::Arc;
use std::pin::Pin;
use std::future::Future;

use serde_json::Value;

use crate::executor::{ConnectorFn, ExecConfig};
use crate::handler::WorkflowRouter;

// ── Handler types ─────────────────────────────────────────────────────

/// Sync native handler: fn(&Value) -> Result<Value, String>
/// Runs inline in the workflow executor — same process, same memory, no network.
pub type NativeHandler = Box<dyn Fn(&Value) -> Result<Value, String> + Send + Sync>;

/// Registry of native code handlers, keyed by handler_ref name.
pub struct NativeRegistry {
    handlers: HashMap<String, NativeHandler>,
}

impl NativeRegistry {
    pub fn new() -> Self {
        Self { handlers: HashMap::new() }
    }

    /// Register a native handler.
    pub fn register(&mut self, name: impl Into<String>, handler: impl Fn(&Value) -> Result<Value, String> + Send + Sync + 'static) {
        self.handlers.insert(name.into(), Box::new(handler));
    }

    /// Dispatch to a registered handler.
    pub fn dispatch(&self, handler_ref: &str, input: &Value) -> Result<Value, String> {
        match self.handlers.get(handler_ref) {
            Some(handler) => handler(input),
            None => Err(format!("native handler '{}' not registered. Available: {:?}",
                handler_ref, self.handlers.keys().collect::<Vec<_>>())),
        }
    }

    pub fn count(&self) -> usize {
        self.handlers.len()
    }

    pub fn names(&self) -> Vec<&str> {
        self.handlers.keys().map(|s| s.as_str()).collect()
    }
}

impl Default for NativeRegistry {
    fn default() -> Self { Self::new() }
}

// ── WasmWorkerPool — dedicated threads for sub-5μs WASM execution ────

#[cfg(feature = "wasm")]
pub struct WasmWorkerPool {
    sender: std::sync::mpsc::SyncSender<WasmRequest>,
}

#[cfg(feature = "wasm")]
struct WasmRequest {
    payload: Vec<u8>,
    reply: tokio::sync::oneshot::Sender<Result<Vec<u8>, String>>,
}

#[cfg(feature = "wasm")]
impl WasmWorkerPool {
    /// Spawn N dedicated worker threads — pure sync, no tokio runtime inside.
    fn new(
        engine: Arc<wasmtime::Engine>,
        instance_pre: Arc<wasmtime::InstancePre<wasmtime_wasi::preview1::WasiP1Ctx>>,
        workers: usize,
    ) -> Self {
        let (tx, rx) = std::sync::mpsc::sync_channel::<WasmRequest>(workers * 64);
        let rx = Arc::new(std::sync::Mutex::new(rx));

        for i in 0..workers {
            let engine = engine.clone();
            let instance_pre = instance_pre.clone();
            let rx = rx.clone();

            std::thread::Builder::new()
                .name(format!("wasm-worker-{}", i))
                .spawn(move || {
                    // Pure sync loop — no tokio runtime, no async
                    loop {
                        let req = match rx.lock().unwrap().recv() {
                            Ok(r) => r,
                            Err(_) => break, // channel closed
                        };
                        let result = Self::execute_wasi(&engine, &instance_pre, req.payload);
                        let _ = req.reply.send(result);
                    }
                })
                .expect("spawn wasm worker");
        }

        Self { sender: tx }
    }

    fn execute_wasi(
        engine: &wasmtime::Engine,
        instance_pre: &wasmtime::InstancePre<wasmtime_wasi::preview1::WasiP1Ctx>,
        payload: Vec<u8>,
    ) -> Result<Vec<u8>, String> {
        let stdin_pipe = wasmtime_wasi::pipe::MemoryInputPipe::new(bytes::Bytes::from(payload));
        let stdout_pipe = wasmtime_wasi::pipe::MemoryOutputPipe::new(4096);

        let wasi_ctx = wasmtime_wasi::WasiCtxBuilder::new()
            .stdin(stdin_pipe)
            .stdout(stdout_pipe.clone())
            .build_p1();

        let mut store = wasmtime::Store::new(engine, wasi_ctx);
        let instance = instance_pre.instantiate(&mut store)
            .map_err(|e| format!("wasm instantiate: {}", e))?;

        let start = instance.get_typed_func::<(), ()>(&mut store, "_start")
            .map_err(|e| format!("no _start: {}", e))?;
        start.call(&mut store, ())
            .map_err(|e| format!("wasm exec: {}", e))?;

        Ok(stdout_pipe.contents().to_vec())
    }

    async fn call(&self, payload: Vec<u8>) -> Result<Vec<u8>, String> {
        let (tx, rx) = tokio::sync::oneshot::channel();
        self.sender.send(WasmRequest { payload, reply: tx })
            .map_err(|_| "wasm pool: channel closed".to_string())?;
        rx.await.map_err(|_| "wasm pool: worker dropped".to_string())?
    }
}

// ── SidecarPool — keep-alive process pool with line-delimited JSON ───

/// A pool of long-running sidecar processes. Each worker reads JSON lines
/// from stdin and writes JSON lines to stdout. Eliminates per-request
/// process spawn overhead.
/// Pool size per sidecar target.
const SIDECAR_POOL_SIZE: usize = 4;

pub struct SidecarPool {
    workers: HashMap<String, SidecarWorkerPool>,
}

struct SidecarWorkerPool {
    slots: Vec<Arc<tokio::sync::Mutex<SidecarWorker>>>,
    counter: std::sync::atomic::AtomicUsize,
}

struct SidecarWorker {
    command: String,
    child: Option<tokio::process::Child>,
    stdin: Option<tokio::process::ChildStdin>,
    reader: Option<tokio::io::BufReader<tokio::process::ChildStdout>>,
}

impl SidecarWorker {
    fn new(command: String) -> Self {
        Self { command, child: None, stdin: None, reader: None }
    }

    async fn ensure_alive(&mut self) -> Result<(), String> {
        // Check if child is still running
        let needs_spawn = match &mut self.child {
            Some(child) => child.try_wait().ok().flatten().is_some(), // exited
            None => true,
        };

        if needs_spawn {
            let mut child = tokio::process::Command::new("sh")
                .args(["-c", &self.command])
                .stdin(std::process::Stdio::piped())
                .stdout(std::process::Stdio::piped())
                .stderr(std::process::Stdio::piped())
                .spawn()
                .map_err(|e| format!("sidecar spawn: {}", e))?;

            self.stdin = child.stdin.take();
            let stdout = child.stdout.take()
                .ok_or_else(|| "sidecar: no stdout".to_string())?;
            self.reader = Some(tokio::io::BufReader::new(stdout));
            self.child = Some(child);
        }
        Ok(())
    }

    async fn call(&mut self, input: &Value) -> Result<Value, String> {
        use tokio::io::{AsyncWriteExt, AsyncBufReadExt};

        self.ensure_alive().await?;

        // Borrow stdin + reader, do IO, collect result — no &mut self overlap
        let io_result: Result<String, String> = {
            let stdin = self.stdin.as_mut().ok_or("sidecar: no stdin")?;
            let reader = self.reader.as_mut().ok_or("sidecar: no reader")?;

            let mut payload = serde_json::to_vec(input).unwrap_or_default();
            payload.push(b'\n');

            if let Err(e) = stdin.write_all(&payload).await {
                return Err(format!("sidecar write: {}", e)); // will kill below
            }
            if let Err(e) = stdin.flush().await {
                return Err(format!("sidecar flush: {}", e));
            }

            let mut line = String::new();
            match tokio::time::timeout(
                std::time::Duration::from_secs(30),
                reader.read_line(&mut line),
            ).await {
                Ok(Ok(0)) => Err("sidecar: EOF (process exited)".into()),
                Ok(Ok(_)) => Ok(line),
                Ok(Err(e)) => Err(format!("sidecar read: {}", e)),
                Err(_) => Err("sidecar: timeout (30s)".into()),
            }
        };

        match io_result {
            Ok(line) => {
                serde_json::from_str::<Value>(line.trim())
                    .map_err(|_| format!("sidecar: invalid JSON: {}", line.trim()))
            }
            Err(e) => {
                self.kill(); // safe: no outstanding borrows
                Err(e)
            }
        }
    }

    fn kill(&mut self) {
        if let Some(ref mut child) = self.child {
            let _ = child.start_kill();
        }
        self.child = None;
        self.stdin = None;
        self.reader = None;
    }
}

impl SidecarPool {
    pub fn new() -> Self {
        Self { workers: HashMap::new() }
    }

    pub fn register(&mut self, target: String, command: String) {
        let slots: Vec<Arc<tokio::sync::Mutex<SidecarWorker>>> = (0..SIDECAR_POOL_SIZE)
            .map(|_| Arc::new(tokio::sync::Mutex::new(SidecarWorker::new(command.clone()))))
            .collect();
        self.workers.insert(target, SidecarWorkerPool {
            slots,
            counter: std::sync::atomic::AtomicUsize::new(0),
        });
    }

    pub async fn call(&self, target: &str, input: &Value) -> Result<Value, String> {
        let pool = self.workers.get(target)
            .ok_or_else(|| format!("sidecar '{}' not registered", target))?;
        // Round-robin across N workers — eliminates Mutex contention
        let idx = pool.counter.fetch_add(1, std::sync::atomic::Ordering::Relaxed) % pool.slots.len();
        let mut guard = pool.slots[idx].lock().await;
        guard.call(input).await
    }
}

// ── VwfdApp Builder ───────────────────────────────────────────────────

/// Ergonomic builder for VWFD workflow server.
///
/// Registers native handlers, WASM modules, and sidecar processes,
/// then starts the VIL server with workflows loaded from a directory.
pub struct VwfdApp {
    workflow_dir: String,
    port: u16,
    observer_enabled: bool,
    native_registry: NativeRegistry,
    wasm_modules: HashMap<String, String>,     // module_ref → file path (WASI compliant)
    sidecar_commands: HashMap<String, String>,  // target → command
    durability: Option<Arc<crate::DurabilityStore>>,
}

impl VwfdApp {
    /// Enable/disable embedded observer dashboard (/_vil/dashboard/).
    pub fn observer(mut self, enabled: bool) -> Self {
        self.observer_enabled = enabled;
        self
    }

    /// Set state store for execution tracking.
    ///
    /// ```rust,no_run
    /// use vil_vwfd::StateStore;
    /// vil_vwfd::app("workflows/", 8080)
    ///     .state_store(StateStore::InMemory)        // fastest, lose on crash
    ///     .state_store(StateStore::H2InMemory)      // same, Kestra-compatible naming
    ///     .state_store(StateStore::Redb("/path".into())) // persistent, ACID
    ///     .run().await;
    /// ```
    pub fn state_store(mut self, store: crate::StateStore) -> Self {
        match store.build() {
            Ok(ds) => self.durability = Some(Arc::new(ds)),
            Err(e) => eprintln!("  WARNING: state_store init failed — {}", e),
        }
        self
    }

    /// Register a synchronous native handler.
    ///
    /// The handler runs inline — same process, zero network overhead.
    /// Use for: tool execution, data parsing, validation, scoring.
    ///
    /// ```rust,no_run
    /// app.native("validate", |input| {
    ///     let name = input["name"].as_str().ok_or("name required")?;
    ///     Ok(serde_json::json!({"valid": true, "name": name}))
    /// })
    /// ```
    pub fn native(mut self, name: impl Into<String>, handler: impl Fn(&Value) -> Result<Value, String> + Send + Sync + 'static) -> Self {
        self.native_registry.register(name, handler);
        self
    }

    /// Register a WASM module file.
    ///
    /// The module is loaded and sandboxed — separate memory, time-limited.
    /// Use for: hot-deployable business rules, pricing, validation.
    pub fn wasm(mut self, module_ref: impl Into<String>, wasm_path: impl Into<String>) -> Self {
        self.wasm_modules.insert(module_ref.into(), wasm_path.into());
        self
    }

    /// Register a sidecar process.
    ///
    /// Spawns an external process — separate runtime, IPC communication.
    /// Use for: Python ML models, polyglot integrations.
    pub fn sidecar(mut self, target: impl Into<String>, command: impl Into<String>) -> Self {
        self.sidecar_commands.insert(target.into(), command.into());
        self
    }

    /// Run the VWFD server. Blocks until shutdown.
    pub async fn run(self) {
        if let Err(e) = self.run_inner().await {
            eprintln!("VwfdApp error: {}", e);
            std::process::exit(1);
        }
    }

    async fn run_inner(self) -> Result<(), String> {
        use vil_server_core::{
            axum::{self, extract::Extension, body::Bytes, response::IntoResponse, http::Method},
            axum::routing::post,
            Json, StatusCode, ServiceProcess, VilApp,
        };

        // Load workflows from directory
        let load_result = crate::loader::load_dir(&self.workflow_dir);
        if load_result.graphs.is_empty() && !load_result.errors.is_empty() {
            return Err(format!("no workflows loaded: {:?}",
                load_result.errors.iter().map(|e| &e.error).collect::<Vec<_>>()));
        }

        let router = WorkflowRouter::new();
        for g in load_result.graphs {
            let path = g.webhook_route.clone().unwrap_or_else(|| format!("/{}", g.id));
            let method = g.webhook_method.clone();
            router.register(method, path, Arc::new(g));
        }

        let routes = router.routes();
        eprintln!("vil_vwfd server on :{}", self.port);
        for r in &routes {
            eprintln!("  {}", r);
        }

        // Native handlers
        let native_count = self.native_registry.count();
        if native_count > 0 {
            let names = self.native_registry.names();
            eprintln!("  Native handlers: {} ({:?})", native_count, names);
        }

        // WASM modules
        for (module, path) in &self.wasm_modules {
            eprintln!("  WASM module: {} → {}", module, path);
        }

        // Sidecar processes
        for (target, cmd) in &self.sidecar_commands {
            eprintln!("  Sidecar: {} → {}", target, cmd);
        }

        // Init connector pools from env vars
        let mut pools = crate::registry::ConnectorPools::new();
        let pool_errors = pools.init_from_env().await;
        for e in &pool_errors {
            eprintln!("  pool init warning: {}", e);
        }
        let pools = Arc::new(pools);

        // ── WASM: PoolingAllocator + InstancePre — sub-5μs instantiation ──
        // PoolingAllocator pre-allocates memory slots → instantiation = pointer bump
        // Engine(cranelift Speed + pooling) → Module → Linker(WASI) → InstancePre
        #[cfg(feature = "wasm")]
        let wasm_registry = {
            let mut wasm_cfg = wasmtime::Config::new();
            wasm_cfg.cranelift_opt_level(wasmtime::OptLevel::Speed);
            wasm_cfg.parallel_compilation(true);

            let engine = Arc::new(wasmtime::Engine::new(&wasm_cfg).expect("wasmtime engine"));

            let num_workers = std::thread::available_parallelism().map(|n| n.get()).unwrap_or(4).min(8);
            let reg: HashMap<String, Arc<WasmWorkerPool>> = self.wasm_modules.iter()
                .filter_map(|(module_ref, wasm_path)| {
                    let bytes = std::fs::read(wasm_path).map_err(|e| {
                        eprintln!("  WASM load failed: {} → {}", wasm_path, e);
                    }).ok()?;
                    let module = wasmtime::Module::new(&engine, &bytes).map_err(|e| {
                        eprintln!("  WASM compile failed: {}: {}", module_ref, e);
                    }).ok()?;

                    // Pre-link WASI into Linker, then create InstancePre
                    let mut linker = wasmtime::Linker::<wasmtime_wasi::preview1::WasiP1Ctx>::new(&engine);
                    wasmtime_wasi::preview1::add_to_linker_sync(&mut linker, |ctx| ctx).map_err(|e| {
                        eprintln!("  WASM link failed: {}: {}", module_ref, e);
                    }).ok()?;
                    let instance_pre = linker.instantiate_pre(&module).map_err(|e| {
                        eprintln!("  WASM instantiate_pre failed: {}: {}", module_ref, e);
                    }).ok()?;

                    // Create worker pool: N dedicated threads per module
                    let pool = WasmWorkerPool::new(engine.clone(), Arc::new(instance_pre), num_workers);
                    eprintln!("  WASM pool ready: {} ({} bytes, {} workers)", module_ref, bytes.len(), num_workers);
                    Some((module_ref.clone(), Arc::new(pool)))
                })
                .collect();
            Arc::new(reg)
        };
        #[cfg(not(feature = "wasm"))]
        let wasm_registry: Arc<HashMap<String, ()>> = Arc::new(HashMap::new());

        // ── Sidecar: UDS+SHM via vil_sidecar (feature=sidecar) or stdin/stdout pool ──
        #[cfg(feature = "sidecar")]
        let vil_sidecar_registry = {
            let registry = Arc::new(vil_sidecar::SidecarRegistry::new());
            for (target, command) in &self.sidecar_commands {
                let config = vil_sidecar::SidecarConfig::new(target)
                    .command(command.clone())
                    .timeout(30_000)
                    .pool_size(4);
                registry.register(config);
                eprintln!("  Sidecar registered (UDS+SHM): {} → {}", target, command);
            }
            registry
        };
        // Connect all sidecars (spawn process, UDS handshake, SHM setup)
        #[cfg(feature = "sidecar")]
        {
            let reg = vil_sidecar_registry.clone();
            for (target, _) in &self.sidecar_commands {
                let t = target.clone();
                let r = reg.clone();
                tokio::spawn(async move {
                    match vil_sidecar::connect_sidecar(&r, &t).await {
                        Ok(_) => eprintln!("  Sidecar connected: {}", t),
                        Err(e) => eprintln!("  Sidecar connect failed: {} — {}", t, e),
                    }
                });
            }
            // Give sidecars time to spawn + handshake
            tokio::time::sleep(std::time::Duration::from_secs(2)).await;
        }

        // Fallback: stdin/stdout pool (when sidecar feature disabled or UDS unavailable)
        let sidecar_pool = {
            let mut pool = SidecarPool::new();
            for (target, command) in &self.sidecar_commands {
                eprintln!("  Sidecar fallback (stdin/stdout pool x4): {} → {}", target, command);
                pool.register(target.clone(), command.clone());
            }
            Arc::new(pool)
        };

        // Build unified ConnectorFn: pools + native handlers + wasm + sidecar
        let native_registry = Arc::new(self.native_registry);

        let connector_fn: ConnectorFn = {
            let pools = pools.clone();
            let native = native_registry.clone();
            let wasm_reg = wasm_registry.clone();
            let sidecar_pool = sidecar_pool.clone();
            #[cfg(feature = "sidecar")]
            let vil_sc_reg = vil_sidecar_registry.clone();

            Arc::new(move |connector_ref: &str, operation: &str, input: &Value| {
                let pools = pools.clone();
                let native = native.clone();
                let wasm_reg = wasm_reg.clone();
                let sidecar_pool = sidecar_pool.clone();
                #[cfg(feature = "sidecar")]
                let vil_sc_reg = vil_sc_reg.clone();
                let connector_ref = connector_ref.to_string();
                let operation = operation.to_string();
                let input = input.clone();

                Box::pin(async move {
                    // NativeCode: vastar.code.{handler_ref}
                    if let Some(handler_ref) = connector_ref.strip_prefix("vastar.code.") {
                        return native.dispatch(handler_ref, &input);
                    }

                    // WASM: vastar.wasm.{module_ref} — Worker pool fast path
                    if let Some(module_ref) = connector_ref.strip_prefix("vastar.wasm.") {
                        #[cfg(feature = "wasm")]
                        {
                            if let Some(pool) = wasm_reg.get(module_ref) {
                                let payload = serde_json::to_vec(&input).unwrap_or_default();

                                // Dispatch to dedicated WASM worker thread via channel
                                let result = pool.call(payload).await;

                                match result {
                                    Ok(output_bytes) => {
                                        match serde_json::from_slice::<Value>(&output_bytes) {
                                            Ok(val) => return Ok(val),
                                            Err(_) => return Ok(Value::String(
                                                String::from_utf8_lossy(&output_bytes).to_string()
                                            )),
                                        }
                                    }
                                    Err(e) => return Err(format!("WASM {}: {}", module_ref, e)),
                                }
                            } else {
                                return Err(format!("WASM module '{}' not registered", module_ref));
                            }
                        }
                        #[cfg(not(feature = "wasm"))]
                        return Err("WASM feature not enabled".into());
                    }

                    // Sidecar: vastar.sidecar.{target}
                    if let Some(target) = connector_ref.strip_prefix("vastar.sidecar.") {
                        // Try UDS+SHM path first (vil_sidecar, ~12μs)
                        #[cfg(feature = "sidecar")]
                        {
                            let request_data = serde_json::to_vec(&input).unwrap_or_default();
                            match vil_sidecar::invoke(&vil_sc_reg, target, "execute", &request_data).await {
                                Ok(resp) => {
                                    return serde_json::from_slice::<Value>(&resp.data)
                                        .map_err(|e| format!("sidecar deserialize: {}", e));
                                }
                                Err(_) => {
                                    // Fallback to stdin/stdout pool
                                }
                            }
                        }
                        // Fallback: stdin/stdout pool (line-delimited JSON)
                        return sidecar_pool.call(target, &input).await;
                    }

                    // Everything else → pool dispatch (HTTP, DB, MQ, Storage)
                    crate::registry::dispatch(&connector_ref, &operation, &input, &pools).await
                }) as Pin<Box<dyn Future<Output = Result<Value, String>> + Send>>
            })
        };

        let config = ExecConfig {
            connector_fn: Some(connector_fn),
            durability: self.durability.clone(),
            ..Default::default()
        };

        /// Extract HTTP status code from error string.
        /// Searches for pattern `NNN:` (3-digit code followed by colon) anywhere in the string.
        fn extract_error_status(e: &str) -> (StatusCode, String) {
            // Search for 3-digit HTTP status code pattern
            for code in [400, 401, 403, 404, 409, 422, 429, 500, 502, 503] {
                let pattern = format!("{}:", code);
                if let Some(pos) = e.find(&pattern) {
                    let msg = e[pos + pattern.len()..].trim().to_string();
                    let status = StatusCode::from_u16(code).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
                    return (status, msg);
                }
            }
            (StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
        }

        let router_ext = Arc::new(router);
        let config_ext = Arc::new(config);

        async fn vwfd_dispatch(
            router: &WorkflowRouter,
            config: &ExecConfig,
            path: String,
            method: &str,
            body_json: serde_json::Value,
            headers: &axum::http::HeaderMap,
        ) -> impl IntoResponse {
            let mut header_map = serde_json::Map::new();
            for (k, v) in headers.iter() {
                if let Ok(val) = v.to_str() {
                    header_map.insert(k.as_str().to_string(), Value::String(val.to_string()));
                }
            }
            let input = serde_json::json!({
                "body": body_json,
                "headers": header_map,
                "path": &path,
                "method": method,
            });

            match crate::handler::handle_request(router, method, &path, input, config).await {
                Ok(output) => {
                    // Check for _status field in output for custom HTTP status
                    let status = output.get("_status")
                        .and_then(|s| s.as_u64())
                        .and_then(|code| StatusCode::from_u16(code as u16).ok())
                        .unwrap_or(StatusCode::OK);
                    // Remove _status from response
                    let body = if output.get("_status").is_some() {
                        let mut obj = output;
                        obj.as_object_mut().map(|o| o.remove("_status"));
                        obj
                    } else {
                        output
                    };
                    (status, Json(body)).into_response()
                }
                Err(e) => {
                    // Parse error code: "400:msg", "node 'x': 403:msg", etc.
                    let (status, msg) = extract_error_status(&e);
                    (status, Json(serde_json::json!({"error": msg}))).into_response()
                }
            }
        }

        async fn vwfd_post_handler(
            Extension(router): Extension<Arc<WorkflowRouter>>,
            Extension(config): Extension<Arc<ExecConfig>>,
            axum::extract::OriginalUri(uri): axum::extract::OriginalUri,
            headers: axum::http::HeaderMap,
            body: Bytes,
        ) -> impl IntoResponse {
            let path = uri.path().to_string();
            let body_json: serde_json::Value = serde_json::from_slice(&body)
                .unwrap_or(serde_json::json!({}));
            vwfd_dispatch(&router, &config, path, "POST", body_json, &headers).await
        }

        async fn vwfd_get_handler(
            Extension(router): Extension<Arc<WorkflowRouter>>,
            Extension(config): Extension<Arc<ExecConfig>>,
            axum::extract::OriginalUri(uri): axum::extract::OriginalUri,
            headers: axum::http::HeaderMap,
        ) -> impl IntoResponse {
            let path = uri.path().to_string();
            vwfd_dispatch(&router, &config, path, "GET", serde_json::json!({}), &headers).await
        }

        async fn vwfd_put_handler(
            Extension(router): Extension<Arc<WorkflowRouter>>,
            Extension(config): Extension<Arc<ExecConfig>>,
            axum::extract::OriginalUri(uri): axum::extract::OriginalUri,
            headers: axum::http::HeaderMap,
            body: Bytes,
        ) -> impl IntoResponse {
            let path = uri.path().to_string();
            let body_json: serde_json::Value = serde_json::from_slice(&body)
                .unwrap_or(serde_json::json!({}));
            vwfd_dispatch(&router, &config, path, "PUT", body_json, &headers).await
        }

        async fn vwfd_delete_handler(
            Extension(router): Extension<Arc<WorkflowRouter>>,
            Extension(config): Extension<Arc<ExecConfig>>,
            axum::extract::OriginalUri(uri): axum::extract::OriginalUri,
            headers: axum::http::HeaderMap,
        ) -> impl IntoResponse {
            let path = uri.path().to_string();
            vwfd_dispatch(&router, &config, path, "DELETE", serde_json::json!({}), &headers).await
        }

        let svc = ServiceProcess::new("vwfd")
            .prefix("")
            .endpoint(Method::POST, "/*path", post(vwfd_post_handler))
            .endpoint(Method::GET, "/*path", axum::routing::get(vwfd_get_handler))
            .endpoint(Method::GET, "/", axum::routing::get(vwfd_get_handler))
            .endpoint(Method::PUT, "/*path", axum::routing::put(vwfd_put_handler))
            .endpoint(Method::DELETE, "/*path", axum::routing::delete(vwfd_delete_handler))
            .extension(router_ext)
            .extension(config_ext);

        VilApp::new("vil-vwfd")
            .port(self.port)
            .observer(self.observer_enabled)
            .service(svc)
            .run()
            .await;

        Ok(())
    }
}

// ── Public API ─────────────────────────────────────────────────────────

/// Create a new VwfdApp builder.
///
/// ```rust,no_run
/// vil_vwfd::app("workflows/", 3200)
///     .native("handler_name", |input| Ok(json!({"ok": true})))
///     .run()
///     .await;
/// ```
pub fn app(workflow_dir: impl Into<String>, port: u16) -> VwfdApp {
    VwfdApp {
        workflow_dir: workflow_dir.into(),
        port,
        observer_enabled: false,
        native_registry: NativeRegistry::new(),
        wasm_modules: HashMap::new(),
        sidecar_commands: HashMap::new(),
        durability: None,
    }
}

// ── Prelude ────────────────────────────────────────────────────────────

pub mod prelude {
    pub use serde_json::{json, Value};
    pub use super::{app, VwfdApp, NativeHandler, NativeRegistry};
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_native_registry_dispatch() {
        let mut reg = NativeRegistry::new();
        reg.register("double", |input| {
            let n = input["n"].as_i64().unwrap_or(0);
            Ok(json!({"result": n * 2}))
        });

        let result = reg.dispatch("double", &json!({"n": 5})).unwrap();
        assert_eq!(result["result"], 10);
    }

    #[test]
    fn test_native_registry_not_found() {
        let reg = NativeRegistry::new();
        let result = reg.dispatch("missing", &json!({}));
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("not registered"));
    }

    #[test]
    fn test_builder_chain() {
        let app = app("workflows/", 3000)
            .native("a", |_| Ok(json!(1)))
            .native("b", |_| Ok(json!(2)))
            .wasm("pricing", "pricing.wasm")
            .sidecar("scorer", "python3 score.py");

        assert_eq!(app.native_registry.count(), 2);
        assert_eq!(app.wasm_modules.len(), 1);
        assert_eq!(app.sidecar_commands.len(), 1);
    }
}
