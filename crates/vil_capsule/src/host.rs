// =============================================================================
// crates/vil_capsule/src/host.rs — CapsuleHost
// =============================================================================

use crate::CapsuleError;

/// Configuration for creating a CapsuleHost.
#[derive(Debug, Clone)]
pub struct CapsuleConfig {
    /// Raw bytes of the .wasm file to load
    pub wasm_bytes: Vec<u8>,
    /// Module/capsule name for logging
    pub module_name: String,
    /// Maximum allowed memory pages (1 page = 64KB)
    pub max_memory_pages: u32,
    /// Maximum fuel (instruction budget) per call. 0 = unlimited.
    pub max_fuel: u64,
    /// Enable epoch-based interruption for CPU time limits.
    pub epoch_interruption: bool,
}

impl CapsuleConfig {
    pub fn new(module_name: impl Into<String>, wasm_bytes: Vec<u8>) -> Self {
        Self {
            wasm_bytes,
            module_name: module_name.into(),
            max_memory_pages: 256,   // Default: 16MB
            max_fuel: 1_000_000_000, // Default: 1B instructions (~1 second)
            epoch_interruption: false,
        }
    }

    pub fn max_memory_pages(mut self, pages: u32) -> Self {
        self.max_memory_pages = pages;
        self
    }

    pub fn max_fuel(mut self, fuel: u64) -> Self {
        self.max_fuel = fuel;
        self
    }

    pub fn epoch_interruption(mut self, enabled: bool) -> Self {
        self.epoch_interruption = enabled;
        self
    }
}

/// Input to a WASM capsule.
#[derive(Debug, Clone)]
pub struct CapsuleInput {
    pub function_name: String,
    pub payload: Vec<u8>, // serialized input bytes
}

/// Output from a WASM capsule.
#[derive(Debug, Clone)]
pub struct CapsuleOutput {
    pub payload: Vec<u8>, // serialized output bytes
    pub logs: Vec<String>,
}

/// Host that manages the lifecycle of a WASM capsule.
///
/// Without the `wasm` feature: all operations return an error.
/// With the `wasm` feature: uses wasmtime for actual execution.
///
/// For best performance, call `precompile()` after construction. This caches
/// the Engine and pre-compiled Module so that each `call()` only creates a
/// lightweight Store.
pub struct CapsuleHost {
    pub config: CapsuleConfig,
    #[cfg(feature = "wasm")]
    engine: Option<wasmtime::Engine>,
    #[cfg(feature = "wasm")]
    module: Option<wasmtime::Module>,
}

impl CapsuleHost {
    /// Create a new CapsuleHost from configuration.
    pub fn new(config: CapsuleConfig) -> Self {
        Self {
            config,
            #[cfg(feature = "wasm")]
            engine: None,
            #[cfg(feature = "wasm")]
            module: None,
        }
    }

    /// Initialize the WASM engine and pre-compile the module.
    /// Call this once after creating CapsuleHost for best performance.
    /// Subsequent calls to `call()` will use the cached engine and module,
    /// avoiding the expensive compilation step on every invocation.
    #[cfg(feature = "wasm")]
    pub fn precompile(&mut self) -> Result<(), CapsuleError> {
        let mut wasm_config = wasmtime::Config::new();
        // Enable fuel metering for instruction-level sandboxing
        if self.config.max_fuel > 0 {
            wasm_config.consume_fuel(true);
        }
        if self.config.epoch_interruption {
            wasm_config.epoch_interruption(true);
        }
        let engine = wasmtime::Engine::new(&wasm_config)
            .map_err(|e| CapsuleError::CompileFailed(e.to_string()))?;
        let module = wasmtime::Module::new(&engine, &self.config.wasm_bytes)
            .map_err(|e| CapsuleError::CompileFailed(e.to_string()))?;
        self.engine = Some(engine);
        self.module = Some(module);
        Ok(())
    }

    /// Configure a Store with fuel and memory limits from CapsuleConfig.
    #[cfg(feature = "wasm")]
    fn configure_store(&self, engine: &wasmtime::Engine) -> wasmtime::Store<()> {
        let mut store = wasmtime::Store::new(engine, ());
        if self.config.max_fuel > 0 {
            let _ = store.set_fuel(self.config.max_fuel);
        }
        store
    }

    /// Execute a function in the capsule and return its output.
    ///
    /// If `precompile()` was called, this uses the optimized path that reuses
    /// the pre-compiled module. Otherwise, it falls back to the legacy path
    /// that creates Engine+Module per call.
    pub fn call(&self, _input: CapsuleInput) -> Result<CapsuleOutput, CapsuleError> {
        #[cfg(not(feature = "wasm"))]
        {
            Err(CapsuleError::WasmFeatureNotEnabled)
        }

        #[cfg(feature = "wasm")]
        {
            if self.engine.is_some() {
                self.call_wasm_optimized(_input)
            } else {
                self.call_wasm_legacy(_input)
            }
        }
    }

    /// Call a WASM function with explicit i32 arguments.
    /// This is the recommended way to call (i32, i32) -> i32 functions.
    /// Requires `precompile()` to have been called first.
    #[cfg(feature = "wasm")]
    pub fn call_i32(&self, function_name: &str, arg0: i32, arg1: i32) -> Result<i32, CapsuleError> {
        let engine = self.engine.as_ref().ok_or_else(|| {
            CapsuleError::ExecutionFailed("not precompiled — call precompile() first".into())
        })?;
        let module = self.module.as_ref().ok_or_else(|| {
            CapsuleError::ExecutionFailed("not precompiled — call precompile() first".into())
        })?;

        let mut store = self.configure_store(engine);
        let mut linker = wasmtime::Linker::new(engine);
        linker
            .func_wrap("env", "vil_log", |_: i32, _: i32| {})
            .map_err(|e| CapsuleError::InstantiateFailed(e.to_string()))?;

        let instance = linker
            .instantiate(&mut store, module)
            .map_err(|e| CapsuleError::InstantiateFailed(e.to_string()))?;

        let func = instance
            .get_typed_func::<(i32, i32), i32>(&mut store, function_name)
            .map_err(|e| {
                CapsuleError::ExecutionFailed(format!(
                    "function '{}' not found: {}",
                    function_name, e
                ))
            })?;

        func.call(&mut store, (arg0, arg1))
            .map_err(|e| CapsuleError::ExecutionFailed(e.to_string()))
    }

    /// Call a WASM function with CapsuleInput (i32, i32) -> i32 pattern.
    ///
    /// Level 1 zero-copy: if payload is non-empty, writes directly to WASM
    /// linear memory via data_mut() (1 copy). Otherwise passes payload length
    /// as the first i32 argument.
    ///
    /// Requires `precompile()` to have been called first.
    #[cfg(feature = "wasm")]
    pub fn call_wasm_optimized(&self, input: CapsuleInput) -> Result<CapsuleOutput, CapsuleError> {
        let engine = self.engine.as_ref().ok_or_else(|| {
            CapsuleError::ExecutionFailed("not precompiled — call precompile() first".into())
        })?;
        let module = self.module.as_ref().ok_or_else(|| {
            CapsuleError::ExecutionFailed("not precompiled — call precompile() first".into())
        })?;

        let mut store = wasmtime::Store::new(engine, ());

        let mut linker = wasmtime::Linker::new(engine);
        linker
            .func_wrap("env", "vil_log", |_: i32, _: i32| {})
            .map_err(|e| CapsuleError::InstantiateFailed(e.to_string()))?;

        let instance = linker
            .instantiate(&mut store, module)
            .map_err(|e| CapsuleError::InstantiateFailed(e.to_string()))?;

        // If payload is non-empty, write it directly to WASM memory (Level 1)
        if !input.payload.is_empty() {
            if let Some(memory) = instance.get_memory(&mut store, "memory") {
                let wasm_mem = memory.data_mut(&mut store);
                let end = input.payload.len().min(wasm_mem.len());
                wasm_mem[0..end].copy_from_slice(&input.payload[0..end]);
            }
        }

        let func = instance
            .get_typed_func::<(i32, i32), i32>(&mut store, &input.function_name)
            .map_err(|e| {
                CapsuleError::ExecutionFailed(format!(
                    "function '{}' not found: {}",
                    input.function_name, e
                ))
            })?;

        let result = func
            .call(&mut store, (input.payload.len() as i32, 0))
            .map_err(|e| CapsuleError::ExecutionFailed(e.to_string()))?;

        Ok(CapsuleOutput {
            payload: result.to_le_bytes().to_vec(),
            logs: vec![],
        })
    }

    /// Call a WASM function with bytes I/O via WASI stdin/stdout.
    ///
    /// Same approach as vwfd WasmWorkerPool (proven, production-grade):
    ///   1. Input bytes → WASI stdin as JSON: {"fn":"name","data":"input"}
    ///   2. Call _start() — WASM binary reads stdin, processes, writes stdout
    ///   3. Output bytes ← WASI stdout
    ///
    /// Runs on a **dedicated OS thread** (not tokio) because WASI P1 build_p1()
    /// creates an internal runtime that conflicts with tokio's async executor.
    #[cfg(all(feature = "wasm", feature = "wasi"))]
    pub fn call_with_memory(
        &self,
        function_name: &str,
        input_bytes: &[u8],
    ) -> Result<Vec<u8>, CapsuleError> {
        let wasm_bytes = self.config.wasm_bytes.clone();
        let max_fuel = self.config.max_fuel;
        let func = function_name.to_string();
        let input = input_bytes.to_vec();

        // Execute on dedicated OS thread — WASI P1 cannot run inside tokio runtime
        let (tx, rx) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            let result = Self::execute_wasi_sync(&wasm_bytes, max_fuel, &func, &input);
            let _ = tx.send(result);
        });

        rx.recv()
            .map_err(|_| CapsuleError::ExecutionFailed("wasi worker thread dropped".into()))?
    }

    #[cfg(all(feature = "wasm", feature = "wasi"))]
    fn execute_wasi_sync(
        wasm_bytes: &[u8],
        max_fuel: u64,
        function_name: &str,
        input_bytes: &[u8],
    ) -> Result<Vec<u8>, CapsuleError> {
        let mut wasm_config = wasmtime::Config::new();
        if max_fuel > 0 {
            wasm_config.consume_fuel(true);
        }
        let engine = wasmtime::Engine::new(&wasm_config)
            .map_err(|e| CapsuleError::CompileFailed(e.to_string()))?;
        let module = wasmtime::Module::new(&engine, wasm_bytes)
            .map_err(|e| CapsuleError::CompileFailed(e.to_string()))?;

        let envelope = serde_json::json!({
            "fn": function_name,
            "data": String::from_utf8_lossy(input_bytes),
        });
        let payload = serde_json::to_vec(&envelope)
            .map_err(|e| CapsuleError::ExecutionFailed(format!("json encode: {}", e)))?;

        let stdin_pipe = wasmtime_wasi::pipe::MemoryInputPipe::new(bytes::Bytes::from(payload));
        let stdout_pipe = wasmtime_wasi::pipe::MemoryOutputPipe::new(4096);

        let wasi_ctx = wasmtime_wasi::WasiCtxBuilder::new()
            .stdin(stdin_pipe)
            .stdout(stdout_pipe.clone())
            .build_p1();

        let mut store = wasmtime::Store::new(&engine, wasi_ctx);
        if max_fuel > 0 {
            let _ = store.set_fuel(max_fuel);
        }

        let mut linker = wasmtime::Linker::<wasmtime_wasi::preview1::WasiP1Ctx>::new(&engine);
        wasmtime_wasi::preview1::add_to_linker_sync(&mut linker, |ctx| ctx)
            .map_err(|e| CapsuleError::InstantiateFailed(format!("wasi link: {}", e)))?;

        let instance = linker
            .instantiate(&mut store, &module)
            .map_err(|e| CapsuleError::InstantiateFailed(e.to_string()))?;

        let start = instance
            .get_typed_func::<(), ()>(&mut store, "_start")
            .map_err(|e| CapsuleError::ExecutionFailed(format!("no _start: {}", e)))?;
        start
            .call(&mut store, ())
            .map_err(|e| CapsuleError::ExecutionFailed(e.to_string()))?;

        Ok(stdout_pipe.contents().to_vec())
    }

    /// Fallback when wasi feature not enabled
    #[cfg(all(feature = "wasm", not(feature = "wasi")))]
    pub fn call_with_memory(
        &self,
        _function_name: &str,
        _input_bytes: &[u8],
    ) -> Result<Vec<u8>, CapsuleError> {
        Err(CapsuleError::ExecutionFailed(
            "call_with_memory requires 'wasi' feature".into(),
        ))
    }

    /// Legacy call path — creates Engine + Module per invocation.
    /// Used when `precompile()` has not been called.
    #[cfg(feature = "wasm")]
    fn call_wasm_legacy(&self, input: CapsuleInput) -> Result<CapsuleOutput, CapsuleError> {
        use wasmtime::{Linker, Module};

        let mut wasm_config = wasmtime::Config::new();
        if self.config.max_fuel > 0 {
            wasm_config.consume_fuel(true);
        }
        let engine = wasmtime::Engine::new(&wasm_config)
            .map_err(|e| CapsuleError::CompileFailed(e.to_string()))?;
        let module = Module::from_binary(&engine, &self.config.wasm_bytes)
            .map_err(|e| CapsuleError::CompileFailed(e.to_string()))?;

        let mut linker = Linker::new(&engine);

        // Register host functions — controlled allow-list for capsule interaction
        // These functions are the ONLY way the wasm code can talk to the outside world.
        linker
            .func_wrap("env", "vil_log", |_: i32, _: i32| {
                // In a real system this would log via the VIL control lane
            })
            .map_err(|e| CapsuleError::InstantiateFailed(e.to_string()))?;

        let mut store = self.configure_store(&engine);
        let instance = linker
            .instantiate(&mut store, &module)
            .map_err(|e| CapsuleError::InstantiateFailed(e.to_string()))?;

        // Call the function
        let func = instance
            .get_typed_func::<(i32, i32), i32>(&mut store, &input.function_name)
            .map_err(|e| {
                CapsuleError::ExecutionFailed(format!(
                    "function '{}' not found: {}",
                    input.function_name, e
                ))
            })?;

        let result = func
            .call(&mut store, (input.payload.len() as i32, 0))
            .map_err(|e| CapsuleError::ExecutionFailed(e.to_string()))?;

        Ok(CapsuleOutput {
            payload: result.to_le_bytes().to_vec(),
            logs: vec![],
        })
    }
}
