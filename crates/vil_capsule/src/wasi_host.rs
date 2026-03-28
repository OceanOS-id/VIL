//! WasiCapsuleHost — WASM capsule with WASI Preview 2 capabilities.
//!
//! Unlike CapsuleHost (pure compute sandbox), WasiCapsuleHost grants
//! controlled access to system resources:
//!   - Filesystem (read-only, scoped to allowed directories)
//!   - Environment variables (allow-listed)
//!   - Clock (monotonic + wall)
//!   - Stdout/Stderr (captured to logs)
//!
//! HTTP outbound is NOT in wasmtime-wasi core — it requires
//! wasmtime-wasi-http (separate crate). For now, we provide the
//! foundation and will add HTTP when wasi-http stabilizes.

use crate::CapsuleError;

/// Capabilities that can be granted to a WASI capsule.
#[derive(Debug, Clone, Default)]
pub struct WasiCapabilities {
    /// Directories the capsule can read from (preopened).
    pub fs_read: Vec<String>,
    /// Directories the capsule can write to (preopened).
    pub fs_write: Vec<String>,
    /// Environment variables visible to the capsule.
    pub env_vars: Vec<(String, String)>,
    /// Program arguments (argv).
    pub args: Vec<String>,
    /// Allow stdout/stderr inheritance.
    pub allow_stdout: bool,
    /// Inherit all environment variables from host.
    pub inherit_env: bool,
}

impl WasiCapabilities {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn fs_read(mut self, path: impl Into<String>) -> Self {
        self.fs_read.push(path.into());
        self
    }

    pub fn fs_write(mut self, path: impl Into<String>) -> Self {
        self.fs_write.push(path.into());
        self
    }

    pub fn env(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.env_vars.push((key.into(), value.into()));
        self
    }

    pub fn arg(mut self, arg: impl Into<String>) -> Self {
        self.args.push(arg.into());
        self
    }

    pub fn stdout(mut self, allow: bool) -> Self {
        self.allow_stdout = allow;
        self
    }

    pub fn inherit_env(mut self, inherit: bool) -> Self {
        self.inherit_env = inherit;
        self
    }
}

/// WASI-aware capsule host with configurable capabilities.
///
/// Uses wasmtime-wasi preview1 compatibility layer for standard WASM modules
/// compiled with `wasm32-wasip1` target.
pub struct WasiCapsuleHost {
    pub config: crate::CapsuleConfig,
    pub capabilities: WasiCapabilities,
    engine: Option<wasmtime::Engine>,
    module: Option<wasmtime::Module>,
}

impl WasiCapsuleHost {
    pub fn new(config: crate::CapsuleConfig, capabilities: WasiCapabilities) -> Self {
        Self {
            config,
            capabilities,
            engine: None,
            module: None,
        }
    }

    /// Pre-compile the WASM module for fast instantiation.
    pub fn precompile(&mut self) -> Result<(), CapsuleError> {
        let engine = wasmtime::Engine::default();
        let module = wasmtime::Module::new(&engine, &self.config.wasm_bytes)
            .map_err(|e| CapsuleError::CompileFailed(e.to_string()))?;
        self.engine = Some(engine);
        self.module = Some(module);
        Ok(())
    }

    /// Build a WasiP1Ctx from our capabilities configuration.
    fn build_wasi_p1_ctx(&self) -> wasmtime_wasi::preview1::WasiP1Ctx {
        let mut builder = wasmtime_wasi::WasiCtxBuilder::new();

        if self.capabilities.allow_stdout {
            builder.inherit_stdio();
        }

        if self.capabilities.inherit_env {
            builder.inherit_env();
        }

        for (k, v) in &self.capabilities.env_vars {
            builder.env(k, v);
        }

        for arg in &self.capabilities.args {
            builder.arg(arg);
        }

        // Preopened directories for filesystem access
        for dir_path in &self.capabilities.fs_read {
            let _ = builder.preopened_dir(
                dir_path,
                dir_path,
                wasmtime_wasi::DirPerms::READ,
                wasmtime_wasi::FilePerms::READ,
            );
        }

        for dir_path in &self.capabilities.fs_write {
            let _ = builder.preopened_dir(
                dir_path,
                dir_path,
                wasmtime_wasi::DirPerms::all(),
                wasmtime_wasi::FilePerms::all(),
            );
        }

        builder.build_p1()
    }

    /// Run the WASI module's `_start` entry point (like a CLI program).
    /// Returns exit code (0 = success).
    pub fn run_start(&self) -> Result<i32, CapsuleError> {
        let engine = self
            .engine
            .as_ref()
            .ok_or_else(|| CapsuleError::ExecutionFailed("not precompiled".into()))?;
        let module = self
            .module
            .as_ref()
            .ok_or_else(|| CapsuleError::ExecutionFailed("not precompiled".into()))?;

        let wasi_ctx = self.build_wasi_p1_ctx();

        let mut linker = wasmtime::Linker::new(engine);
        wasmtime_wasi::preview1::add_to_linker_sync(&mut linker, |ctx| ctx)
            .map_err(|e| CapsuleError::InstantiateFailed(e.to_string()))?;

        let mut store = wasmtime::Store::new(engine, wasi_ctx);

        let instance = linker
            .instantiate(&mut store, module)
            .map_err(|e| CapsuleError::InstantiateFailed(e.to_string()))?;

        let start = instance
            .get_typed_func::<(), ()>(&mut store, "_start")
            .map_err(|e| CapsuleError::ExecutionFailed(format!("no _start function: {}", e)))?;

        match start.call(&mut store, ()) {
            Ok(()) => Ok(0),
            Err(e) => {
                if let Some(exit) = e.downcast_ref::<wasmtime_wasi::I32Exit>() {
                    Ok(exit.0)
                } else {
                    Err(CapsuleError::ExecutionFailed(e.to_string()))
                }
            }
        }
    }

    /// Call a named export function with (i32, i32) -> i32 signature,
    /// with WASI capabilities available to the WASM module.
    pub fn call_i32(&self, function_name: &str, arg0: i32, arg1: i32) -> Result<i32, CapsuleError> {
        let engine = self
            .engine
            .as_ref()
            .ok_or_else(|| CapsuleError::ExecutionFailed("not precompiled".into()))?;
        let module = self
            .module
            .as_ref()
            .ok_or_else(|| CapsuleError::ExecutionFailed("not precompiled".into()))?;

        let wasi_ctx = self.build_wasi_p1_ctx();

        let mut linker = wasmtime::Linker::new(engine);
        wasmtime_wasi::preview1::add_to_linker_sync(&mut linker, |ctx| ctx)
            .map_err(|e| CapsuleError::InstantiateFailed(e.to_string()))?;

        // Also add vil_log host function
        linker
            .func_wrap("env", "vil_log", |_: i32, _: i32| {})
            .map_err(|e| CapsuleError::InstantiateFailed(e.to_string()))?;

        let mut store = wasmtime::Store::new(engine, wasi_ctx);

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
}
