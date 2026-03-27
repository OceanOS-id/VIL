# WASM FaaS

`vil_capsule` provides WASM-based Function-as-a-Service with hot-reload and pooling.

## CapsuleHost

```rust
use vil_capsule::prelude::*;

let host = CapsuleHost::new()
    .wasm_dir("./wasm_modules/")
    .pool_size(4)
    .hot_reload(true)
    .build()
    .await?;
```

## Register as Service

```rust
let service = ServiceProcess::new("faas")
    .extension(host.clone())
    .endpoint(Method::POST, "/invoke/:module", post(invoke_handler));

VilApp::new("wasm-faas")
    .port(8080)
    .service(service)
    .run()
    .await;
```

## Invoke Handler

```rust
#[vil_handler(shm)]
async fn invoke_handler(
    ctx: ServiceCtx,
    Path(module): Path<String>,
    slice: ShmSlice,
) -> VilResponse<serde_json::Value> {
    let host = ctx.state::<CapsuleHost>();
    let input = slice.as_bytes();
    let output = host.invoke(&module, input).await?;
    let result: serde_json::Value = serde_json::from_slice(&output)?;
    VilResponse::ok(result)
}
```

## WasmPool

Pre-warmed pool of WASM instances for low-latency invocation:

```rust
let pool = WasmPool::new()
    .module_path("./wasm_modules/transform.wasm")
    .instances(8)         // Pre-warm 8 instances
    .max_memory(16_mb)    // Per-instance memory limit
    .timeout(Duration::from_secs(5))
    .build()
    .await?;

let result = pool.invoke(input_bytes).await?;
```

## Hot-Reload

When enabled, file changes in `wasm_dir` trigger automatic module reloading:

```rust
let host = CapsuleHost::new()
    .wasm_dir("./wasm_modules/")
    .hot_reload(true)       // Watch for .wasm file changes
    .reload_debounce(500)   // Debounce in ms
    .build()
    .await?;
```

## Writing a WASM Module

```rust
// lib.rs (compiled to wasm32-wasi)
#[no_mangle]
pub extern "C" fn process(input_ptr: *const u8, input_len: u32) -> u64 {
    let input = unsafe { std::slice::from_raw_parts(input_ptr, input_len as usize) };
    let data: serde_json::Value = serde_json::from_slice(input).unwrap();
    let output = transform(data);
    let bytes = serde_json::to_vec(&output).unwrap();
    // Return packed (ptr, len)
    pack_result(&bytes)
}
```

## Build WASM Module

```bash
cargo build --target wasm32-wasi --release
cp target/wasm32-wasi/release/my_module.wasm ./wasm_modules/
```

## CapsuleHost vs WasmPool

| Feature | CapsuleHost | WasmPool |
|---------|-------------|----------|
| Multi-module | Yes | Single module |
| Hot-reload | Yes | No |
| Use case | FaaS platform | Dedicated worker |

> Reference: docs/vil/006-VIL-Developer_Guide-CLI-Deployment.md
