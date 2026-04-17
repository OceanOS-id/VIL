// ╔════════════════════════════════════════════════════════════╗
// ║  021 — Business Rules Engine (WASM Sandboxed)             ║
// ╠════════════════════════════════════════════════════════════╣
// ║  Domain:   Commerce — Pricing & Validation Rules           ║
// ║  Pattern:  VX_APP                                           ║
// ║  Token:    N/A (HTTP server)                                ║
// ║  Features: ShmSlice, ServiceCtx, WasmFaaS, Level 1 Zero-Copy║
// ╠════════════════════════════════════════════════════════════╣
// ║  Business: Executes business rules (pricing, validation,   ║
// ║  data transformation) in sandboxed WASM modules. Rules     ║
// ║  are compiled from Rust to .wasm, hot-deployable without   ║
// ║  server restart. Pre-warmed instance pool ensures <1ms     ║
// ║  cold-start latency. Memory-isolated — a buggy rule        ║
// ║  cannot crash the host server.                              ║
// ╚════════════════════════════════════════════════════════════╝
// Business Rules Engine — Real WASM FaaS with Pre-compiled Module Pool
// =============================================================================
//
// Demonstrates REAL WASM execution via wasmtime:
//   - Load actual .wasm files compiled from Rust
//   - WasmFaaSRegistry with pre-warmed instance pools
//   - Pricing: calculate_price, apply_tax, bulk_discount
//   - Validation: validate_order, validate_age, validate_quantity
//   - Transform: to_uppercase, reverse_bytes, count_vowels (memory I/O)
//
// Prerequisites:
//   cd wasm-modules && bash build-wasm.sh
//
// Run:
//   cargo run -p basic-usage-wasm-faas
//
// Test:
//   curl http://localhost:8080/
//   curl http://localhost:8080/wasm/modules
//   curl -X POST http://localhost:8080/wasm/pricing \
//     -H 'Content-Type: application/json' \
//     -d '{"function":"calculate_price","args":[1000, 15]}'
//   curl -X POST http://localhost:8080/wasm/validation \
//     -H 'Content-Type: application/json' \
//     -d '{"function":"validate_order","args":[500, 1000]}'
//   curl -X POST http://localhost:8080/wasm/transform \
//     -H 'Content-Type: application/json' \
//     -d '{"function":"to_uppercase","input":"hello vil"}'

use std::sync::Arc;
use vil_capsule::{WasmFaaSConfig, WasmFaaSRegistry};
use vil_server::prelude::*;

// ── Semantic Types ──

#[derive(Clone, Debug, Serialize, Deserialize, VilModel)]
/// Server info — lists all deployed WASM business rule modules.
struct ServerInfo {
    name: String,
    description: String,
    wasm_modules: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, VilModel)]
/// Module metadata — pool size, memory limit, timeout for operations monitoring.
struct WasmModuleInfo {
    name: String,
    pool_size: usize,
    memory_limit_mb: u64,
    timeout_ms: u64,
}

#[derive(Clone, Debug, Deserialize)]
/// WASM invocation request — function name + integer arguments.
struct InvokeArgsRequest {
    function: String,
    args: Vec<i32>,
}

#[derive(Clone, Debug, Deserialize)]
/// WASM memory invocation — function name + string input (for transforms).
struct InvokeMemoryRequest {
    function: String,
    input: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, VilModel)]
/// WASM execution result — includes module name, function, result, and execution type.
struct InvokeResult {
    module: String,
    function: String,
    result: serde_json::Value,
    execution: String,
}

// ── Handlers ──────────────────────────────────────────────────────────────
// Each handler retrieves the WASM registry from ServiceCtx shared state.
// The registry manages pre-warmed module instance pools for <1ms cold-start.
// WASM isolation ensures a buggy pricing rule cannot crash the server.

/// GET / — lists available WASM business rule modules
async fn index(ctx: ServiceCtx) -> VilResponse<ServerInfo> {
    let registry = ctx
        .state::<Arc<WasmFaaSRegistry>>()
        .expect("WasmFaaSRegistry");
    VilResponse::ok(ServerInfo {
        name: "WASM FaaS Example — Real Execution".into(),
        description: "Loads real .wasm modules and executes functions via wasmtime".into(),
        wasm_modules: registry.names(),
    })
}

/// GET /wasm/modules — inventory of deployed business rule modules with pool stats.
/// Shows pool size, memory limits, and timeout per module — useful for ops monitoring.
async fn list_modules(ctx: ServiceCtx) -> VilResponse<Vec<WasmModuleInfo>> {
    let registry = ctx
        .state::<Arc<WasmFaaSRegistry>>()
        .expect("WasmFaaSRegistry");
    let modules: Vec<WasmModuleInfo> = registry
        .names()
        .into_iter()
        .filter_map(|name| {
            registry.get(&name).map(|pool| WasmModuleInfo {
                name,
                pool_size: pool.size(),
                memory_limit_mb: pool.config.memory_limit_bytes() / (1024 * 1024),
                timeout_ms: pool.config.timeout_ms,
            })
        })
        .collect();
    VilResponse::ok(modules)
}

/// POST /wasm/pricing — execute pricing business rules in WASM sandbox.
/// Functions: calculate_price (base + quantity), apply_discount (percentage).
/// WASM isolation ensures a buggy pricing formula cannot crash the server.
async fn invoke_pricing(
    ctx: ServiceCtx,
    body: ShmSlice,
) -> Result<VilResponse<InvokeResult>, VilError> {
    let req: InvokeArgsRequest = body.json().expect("invalid JSON body");
    let registry = ctx
        .state::<Arc<WasmFaaSRegistry>>()
        .expect("WasmFaaSRegistry");
    let pool = registry
        .get("pricing")
        .ok_or_else(|| VilError::not_found("pricing module not loaded"))?;

    let arg0 = req.args.first().copied().unwrap_or(0);
    let arg1 = req.args.get(1).copied().unwrap_or(0);

    let result = pool
        .call_i32(&req.function, arg0, arg1)
        .map_err(|e| VilError::internal(format!("WASM execution failed: {}", e)))?;

    Ok(VilResponse::ok(InvokeResult {
        module: "pricing".into(),
        function: req.function,
        result: serde_json::json!({ "value": result, "args": [arg0, arg1] }),
        execution: "wasm-precompiled".into(),
    }))
}

/// POST /wasm/validation — execute order/payment validation rules.
/// Returns valid=true/false. Rules are hot-deployable without server restart.
async fn invoke_validation(
    ctx: ServiceCtx,
    body: ShmSlice,
) -> Result<VilResponse<InvokeResult>, VilError> {
    let req: InvokeArgsRequest = body.json().expect("invalid JSON body");
    let registry = ctx
        .state::<Arc<WasmFaaSRegistry>>()
        .expect("WasmFaaSRegistry");
    let pool = registry
        .get("validation")
        .ok_or_else(|| VilError::not_found("validation module not loaded"))?;

    let arg0 = req.args.first().copied().unwrap_or(0);
    let arg1 = req.args.get(1).copied().unwrap_or(0);

    let result = pool
        .call_i32(&req.function, arg0, arg1)
        .map_err(|e| VilError::internal(format!("WASM execution failed: {}", e)))?;

    Ok(VilResponse::ok(InvokeResult {
        module: "validation".into(),
        function: req.function,
        result: serde_json::json!({ "valid": result == 1, "raw": result, "args": [arg0, arg1] }),
        execution: "wasm-precompiled".into(),
    }))
}

/// POST /wasm/transform — execute data transformation rules (uppercase, reverse, etc.).
///
/// Zero-copy data flow (Level 1):
///   1. HTTP body arrives in ExchangeHeap via ShmSlice (0 copy)
///   2. body.json() deserializes from SHM (SIMD JSON)
///   3. call_with_memory() writes input directly to WASM linear memory
///      via data_mut() — 1 copy (SHM → WASM), no intermediate buffer
///   4. WASM executes transform function in sandbox
///   5. Result read via data() direct slice — 0 copy within host
///
/// This is the same technique used by Fastly Compute@Edge for
/// near-zero-copy host↔WASM data transfer.
async fn invoke_transform(
    ctx: ServiceCtx,
    body: ShmSlice,
) -> Result<VilResponse<InvokeResult>, VilError> {
    let req: InvokeMemoryRequest = body.json().expect("invalid JSON body");
    let registry = ctx
        .state::<Arc<WasmFaaSRegistry>>()
        .expect("WasmFaaSRegistry");
    let pool = registry
        .get("transform")
        .ok_or_else(|| VilError::not_found("transform module not loaded"))?;

    // Level 1 zero-copy: input bytes written directly to WASM linear memory
    // via memory.data_mut() slice. Response read as direct slice reference.
    let output_bytes = pool
        .call_with_memory(&req.function, req.input.as_bytes())
        .map_err(|e| VilError::internal(format!("WASM memory execution failed: {}", e)))?;

    let output_str = String::from_utf8_lossy(&output_bytes).to_string();

    Ok(VilResponse::ok(InvokeResult {
        module: "transform".into(),
        function: req.function.clone(),
        result: if req.function == "count_vowels" {
            // count_vowels returns a number, not transformed text
            let count = if output_bytes.len() >= 4 {
                i32::from_le_bytes([
                    output_bytes[0],
                    output_bytes[1],
                    output_bytes[2],
                    output_bytes[3],
                ])
            } else {
                output_bytes.len() as i32
            };
            serde_json::json!({ "input": req.input, "vowel_count": count })
        } else {
            serde_json::json!({ "input": req.input, "output": output_str })
        },
        execution: "wasm-memory-io".into(),
    }))
}

// ── Load WASM bytes ──

// Load pre-compiled WASM module bytes from disk. Build with: cd wasm-modules && bash build-wasm.sh
fn load_wasm_bytes(module_name: &str) -> Vec<u8> {
    let paths = [
        format!("examples/021-basic-wasm-faas/wasm-modules/out/{}.wasm", module_name),
        format!("wasm-modules/out/{}.wasm", module_name),
        format!("../../wasm-modules/out/{}.wasm", module_name),
    ];

    for path in &paths {
        if let Ok(bytes) = std::fs::read(path) {
            println!(
                "  Loaded {}.wasm ({} bytes) from {}",
                module_name,
                bytes.len(),
                path
            );
            return bytes;
        }
    }

    panic!(
        "ERROR: {}.wasm not found. Build WASM modules first:\n  cd wasm-modules && bash build-wasm.sh",
        module_name
    );
}

#[tokio::main]
// ── Main — assemble the Business Rules Engine service ───────────────
async fn main() {
    println!("=== VIL WASM FaaS — Loading Real WASM Modules ===\n");

    // Create the WASM module registry — manages pre-compiled module pools
    let registry = Arc::new(WasmFaaSRegistry::new());

    // Register pricing module (pool of 4 pre-warmed, pre-compiled instances)
    registry.register(
        WasmFaaSConfig::new("pricing", load_wasm_bytes("pricing"))
            .pool_size(4)
            .timeout_ms(5000)
            .max_memory_pages(256),
    );

    // Register validation module
    registry.register(
        WasmFaaSConfig::new("validation", load_wasm_bytes("validation"))
            .pool_size(4)
            .timeout_ms(2000)
            .max_memory_pages(256),
    );

    // Register transform module (needs memory export for I/O)
    registry.register(
        WasmFaaSConfig::new("transform", load_wasm_bytes("transform"))
            .pool_size(4)
            .timeout_ms(5000)
            .max_memory_pages(512),
    );

    println!(
        "\nRegistered {} WASM modules: {:?}\n",
        registry.count(),
        registry.names()
    );

    let wasm_svc = ServiceProcess::new("wasm-faas")
        .endpoint(Method::GET, "/", get(index))
        .endpoint(Method::GET, "/wasm/modules", get(list_modules))
        .endpoint(Method::POST, "/wasm/pricing", post(invoke_pricing))
        .endpoint(Method::POST, "/wasm/validation", post(invoke_validation))
        .endpoint(Method::POST, "/wasm/transform", post(invoke_transform))
        .state(registry);

    VilApp::new("wasm-faas-example")
        .port(8080)
        .service(wasm_svc)
        .run()
        .await;
}
