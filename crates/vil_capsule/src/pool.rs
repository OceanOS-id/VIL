// =============================================================================
// WASM Instance Pool — Pre-warmed WASM instances for FaaS-style execution
// =============================================================================
//
// Amortizes cold start by maintaining N pre-warmed CapsuleHost instances.
// Requests are dispatched round-robin across the pool.

use crate::{CapsuleConfig, CapsuleError, CapsuleHost, CapsuleInput, CapsuleOutput};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

/// Configuration for a WASM FaaS module.
#[derive(Debug, Clone)]
pub struct WasmFaaSConfig {
    /// Module name (for registry lookup and metrics).
    pub module_name: String,
    /// WASM bytecode.
    pub wasm_bytes: Vec<u8>,
    /// Maximum memory pages (1 page = 64KB). Default: 256 (16MB).
    pub max_memory_pages: u32,
    /// Number of pre-warmed instances in the pool. Default: 4.
    pub pool_size: usize,
    /// Execution timeout in milliseconds. Default: 5000.
    pub timeout_ms: u64,
}

impl WasmFaaSConfig {
    pub fn new(module_name: impl Into<String>, wasm_bytes: Vec<u8>) -> Self {
        Self {
            module_name: module_name.into(),
            wasm_bytes,
            max_memory_pages: 256,
            pool_size: 4,
            timeout_ms: 5000,
        }
    }

    pub fn pool_size(mut self, size: usize) -> Self {
        self.pool_size = size;
        self
    }

    pub fn timeout_ms(mut self, ms: u64) -> Self {
        self.timeout_ms = ms;
        self
    }

    pub fn max_memory_pages(mut self, pages: u32) -> Self {
        self.max_memory_pages = pages;
        self
    }

    /// Memory limit in bytes.
    pub fn memory_limit_bytes(&self) -> u64 {
        self.max_memory_pages as u64 * 65536
    }
}

/// Pool of pre-warmed WASM instances for round-robin dispatch.
pub struct WasmPool {
    instances: Vec<CapsuleHost>,
    counter: AtomicUsize,
    pub config: WasmFaaSConfig,
}

impl WasmPool {
    /// Create a new pool with N pre-warmed instances.
    /// When the `wasm` feature is enabled, each instance is pre-compiled
    /// so that subsequent calls avoid the expensive compilation step.
    pub fn new(config: WasmFaaSConfig) -> Self {
        let mut instances = Vec::with_capacity(config.pool_size);
        for _ in 0..config.pool_size {
            let capsule_config = CapsuleConfig::new(
                &config.module_name,
                config.wasm_bytes.clone(),
            ).max_memory_pages(config.max_memory_pages);
            #[allow(unused_mut)]
            let mut host = CapsuleHost::new(capsule_config);
            #[cfg(feature = "wasm")]
            {
                let _ = host.precompile();
            }
            instances.push(host);
        }
        Self {
            instances,
            counter: AtomicUsize::new(0),
            config,
        }
    }

    /// Dispatch a function call to the next available instance (round-robin).
    pub fn call(&self, input: CapsuleInput) -> Result<CapsuleOutput, CapsuleError> {
        if self.instances.is_empty() {
            return Err(CapsuleError::ExecutionFailed("empty pool".into()));
        }
        let idx = self.counter.fetch_add(1, Ordering::Relaxed) % self.instances.len();
        self.instances[idx].call(input)
    }

    /// Dispatch a function call with memory I/O to the next available instance.
    /// Requires the `wasm` feature and pre-compiled instances.
    #[cfg(feature = "wasm")]
    pub fn call_with_memory(
        &self,
        function_name: &str,
        input_bytes: &[u8],
    ) -> Result<Vec<u8>, CapsuleError> {
        if self.instances.is_empty() {
            return Err(CapsuleError::ExecutionFailed("empty pool".into()));
        }
        let idx = self.counter.fetch_add(1, Ordering::Relaxed) % self.instances.len();
        self.instances[idx].call_with_memory(function_name, input_bytes)
    }

    /// Dispatch a typed i32 function call: (i32, i32) -> i32.
    /// This is the recommended API for calling WASM functions with explicit arguments.
    #[cfg(feature = "wasm")]
    pub fn call_i32(
        &self,
        function_name: &str,
        arg0: i32,
        arg1: i32,
    ) -> Result<i32, CapsuleError> {
        if self.instances.is_empty() {
            return Err(CapsuleError::ExecutionFailed("empty pool".into()));
        }
        let idx = self.counter.fetch_add(1, Ordering::Relaxed) % self.instances.len();
        self.instances[idx].call_i32(function_name, arg0, arg1)
    }

    /// Number of instances in the pool.
    pub fn size(&self) -> usize {
        self.instances.len()
    }

    /// Module name.
    pub fn module_name(&self) -> &str {
        &self.config.module_name
    }
}

/// Registry of WASM FaaS modules (module_name → WasmPool).
pub struct WasmFaaSRegistry {
    modules: dashmap::DashMap<String, Arc<WasmPool>>,
}

impl WasmFaaSRegistry {
    pub fn new() -> Self {
        Self {
            modules: dashmap::DashMap::new(),
        }
    }

    /// Register a WASM module and create its pool.
    pub fn register(&self, config: WasmFaaSConfig) -> Arc<WasmPool> {
        let name = config.module_name.clone();
        let pool = Arc::new(WasmPool::new(config));
        self.modules.insert(name, pool.clone());
        pool
    }

    /// Get a module's pool by name.
    pub fn get(&self, name: &str) -> Option<Arc<WasmPool>> {
        self.modules.get(name).map(|v| v.value().clone())
    }

    /// Remove a module from the registry.
    pub fn remove(&self, name: &str) -> Option<Arc<WasmPool>> {
        self.modules.remove(name).map(|(_, v)| v)
    }

    /// List all registered module names.
    pub fn names(&self) -> Vec<String> {
        self.modules.iter().map(|e| e.key().clone()).collect()
    }

    /// Number of registered modules.
    pub fn count(&self) -> usize {
        self.modules.len()
    }
}

impl Default for WasmFaaSRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_faas_config_defaults() {
        let cfg = WasmFaaSConfig::new("test", vec![0x00]);
        assert_eq!(cfg.pool_size, 4);
        assert_eq!(cfg.timeout_ms, 5000);
        assert_eq!(cfg.max_memory_pages, 256);
        assert_eq!(cfg.memory_limit_bytes(), 256 * 65536);
    }

    #[test]
    fn test_faas_config_builder() {
        let cfg = WasmFaaSConfig::new("test", vec![0x00])
            .pool_size(8)
            .timeout_ms(10000)
            .max_memory_pages(512);
        assert_eq!(cfg.pool_size, 8);
        assert_eq!(cfg.timeout_ms, 10000);
    }

    #[test]
    fn test_pool_creation() {
        let cfg = WasmFaaSConfig::new("test", vec![0x00]).pool_size(3);
        let pool = WasmPool::new(cfg);
        assert_eq!(pool.size(), 3);
        assert_eq!(pool.module_name(), "test");
    }

    #[test]
    fn test_pool_call_no_wasm_feature() {
        let cfg = WasmFaaSConfig::new("test", vec![0x00]).pool_size(2);
        let pool = WasmPool::new(cfg);
        let input = CapsuleInput {
            function_name: "process".into(),
            payload: vec![42],
        };
        // Without wasm feature, should return WasmFeatureNotEnabled
        #[cfg(not(feature = "wasm"))]
        {
            let result = pool.call(input);
            assert!(matches!(result, Err(CapsuleError::WasmFeatureNotEnabled)));
        }
    }

    #[test]
    fn test_registry() {
        let reg = WasmFaaSRegistry::new();
        let cfg = WasmFaaSConfig::new("pricing", vec![0x00]).pool_size(2);
        reg.register(cfg);

        assert_eq!(reg.count(), 1);
        assert!(reg.get("pricing").is_some());
        assert!(reg.get("missing").is_none());

        let names = reg.names();
        assert_eq!(names.len(), 1);
        assert!(names.contains(&"pricing".to_string()));

        reg.remove("pricing");
        assert_eq!(reg.count(), 0);
    }
}
