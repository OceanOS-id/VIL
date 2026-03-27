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
}

impl CapsuleConfig {
    pub fn new(module_name: impl Into<String>, wasm_bytes: Vec<u8>) -> Self {
        Self {
            wasm_bytes,
            module_name: module_name.into(),
            max_memory_pages: 256, // Default: 16MB
        }
    }

    pub fn max_memory_pages(mut self, pages: u32) -> Self {
        self.max_memory_pages = pages;
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
        let engine = wasmtime::Engine::default();
        let module = wasmtime::Module::new(&engine, &self.config.wasm_bytes)
            .map_err(|e| CapsuleError::CompileFailed(e.to_string()))?;
        self.engine = Some(engine);
        self.module = Some(module);
        Ok(())
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

        let mut store = wasmtime::Store::new(engine, ());
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
                CapsuleError::ExecutionFailed(format!("function '{}' not found: {}", function_name, e))
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

    /// Call a WASM function with direct memory I/O (Level 1 zero-copy).
    ///
    /// Uses `memory.data_mut()` for direct slice access to WASM linear memory,
    /// eliminating intermediate buffers. Total: 1 copy (input → WASM memory).
    /// Response is read as a zero-copy slice reference.
    ///
    /// Protocol:
    ///   1. Direct-write `input_bytes` to WASM linear memory at offset 0
    ///   2. Call `function_name(ptr=0, len=input_bytes.len())` → result_len
    ///   3. Direct-read `result_len` bytes from WASM memory at offset 1024
    ///
    /// This is the same technique used by Fastly Compute@Edge and Cloudflare
    /// Workers for near-zero-copy host↔WASM data transfer. The WASM linear
    /// memory is a contiguous mmap region in the host process — `data_mut()`
    /// returns a direct mutable slice, bypassing wasmtime's copy-based
    /// `memory.write()` / `memory.read()` API.
    ///
    /// Requires `precompile()` to have been called first.
    #[cfg(feature = "wasm")]
    pub fn call_with_memory(
        &self,
        function_name: &str,
        input_bytes: &[u8],
    ) -> Result<Vec<u8>, CapsuleError> {
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

        // Get the memory export from the WASM module
        let memory = instance.get_memory(&mut store, "memory").ok_or_else(|| {
            CapsuleError::ExecutionFailed("WASM module has no 'memory' export".into())
        })?;

        // ── Level 1 Zero-Copy: Direct slice access ──────────────────
        //
        // WASM linear memory is a contiguous mmap region in host address space.
        // memory.data_mut() returns &mut [u8] — a direct mutable slice.
        // This bypasses wasmtime's memory.write() which does an extra memcpy
        // through an intermediate buffer.
        //
        // Copy count:
        //   OLD: input_bytes → staging buffer → WASM memory (2 copies for input)
        //   NEW: input_bytes → WASM memory directly (1 copy for input)

        let wasm_mem = memory.data_mut(&mut store);

        // Bounds check: ensure WASM memory is large enough
        let input_end = input_bytes.len();
        if input_end > wasm_mem.len() {
            return Err(CapsuleError::ExecutionFailed(format!(
                "Input ({} bytes) exceeds WASM memory ({} bytes)",
                input_bytes.len(),
                wasm_mem.len()
            )));
        }

        // SINGLE COPY: input bytes directly into WASM linear memory at offset 0.
        // This is the only copy in the entire host→WASM path.
        wasm_mem[0..input_end].copy_from_slice(input_bytes);

        // Call WASM function: function_name(ptr=0, len=input_bytes.len()) → result_len
        let func = instance
            .get_typed_func::<(i32, i32), i32>(&mut store, function_name)
            .map_err(|e| {
                CapsuleError::ExecutionFailed(format!(
                    "function '{}' not found: {}",
                    function_name, e
                ))
            })?;

        let result_len = func
            .call(&mut store, (0, input_bytes.len() as i32))
            .map_err(|e| CapsuleError::ExecutionFailed(e.to_string()))?;

        // ── Zero-copy read: direct slice from WASM memory ───────────
        //
        // WASM wrote its output at offset 1024. We read it as a direct
        // slice reference — no intermediate buffer, no memcpy.
        // The .to_vec() at the end is needed because the slice borrows
        // from Store which we're returning from. In a pooled scenario
        // (VFlow Level 2), this could be eliminated entirely.

        let result_start = 1024usize;
        let result_end = result_start + result_len as usize;

        let wasm_mem = memory.data(&store);
        if result_end > wasm_mem.len() {
            return Err(CapsuleError::ExecutionFailed(format!(
                "Result region ({}-{}) exceeds WASM memory ({} bytes)",
                result_start, result_end, wasm_mem.len()
            )));
        }

        // Direct slice read — zero copy within host. The final .to_vec()
        // is the ownership boundary (caller needs owned data). In VFlow
        // Level 2, this is eliminated via MemoryCreator + SHM mapping.
        Ok(wasm_mem[result_start..result_end].to_vec())
    }

    /// Legacy call path — creates Engine + Module per invocation.
    /// Used when `precompile()` has not been called.
    #[cfg(feature = "wasm")]
    fn call_wasm_legacy(&self, input: CapsuleInput) -> Result<CapsuleOutput, CapsuleError> {
        use wasmtime::{Engine, Linker, Module, Store};

        let engine = Engine::default();
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

        let mut store = Store::new(&engine, ());
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
