# VIL Developer Guide — Part 11: Custom Code (Native, WASM, Sidecar)

**Series:** VIL Developer Guide (11 of 11)
**Previous:** [Part 10 — Observer Dashboard](./010-VIL-Developer_Guide-Observer-Dashboard.md)
**Last updated:** 2026-04-07

---

## 1. The Zero-Plumbing Principle

VIL Concept P4 states: **"Generated Plumbing, Human-Written Logic."**

Before these macros, using WASM or sidecar required manual plumbing:

```rust
// OLD: manual plumbing (DO NOT USE)
let registry = Arc::new(WasmFaaSRegistry::new());
registry.register(WasmFaaSConfig::new("pricing", load_wasm_bytes("pricing.wasm"))
    .pool_size(4).timeout_ms(5000));
vil_capsule::bridge::init_wasm_registry(registry.clone());
// ... 15+ lines of setup before you can call a function
```

Now:

```rust
// NEW: zero plumbing
#[vil_wasm(module = "pricing")]
fn calculate_price(base_cents: i32, qty: i32) -> i32 {
    // Your logic. VIL handles everything else.
}
```

---

## 2. `#[vil_wasm]` — Sandboxed WASM Execution

### When to Use

- Business rules that change frequently (pricing, validation, scoring)
- Untrusted or third-party code that needs memory isolation
- Hot-deployable logic without server restart
- **NOT for pure Rust** — native is always faster with no isolation overhead

### How It Works

```
Developer writes:           VIL generates:
┌─────────────────┐        ┌──────────────────────────────────┐
│ #[vil_wasm]     │        │ 1. Bridge fn (same signature)    │
│ fn calc(a, b)   │ ────→  │ 2. Pool lazy-init on first call  │
│   { logic }     │        │ 3. .wasm auto-load from disk     │
└─────────────────┘        │ 4. Native fallback if no .wasm   │
                           │ 5. Metadata for introspection    │
                           └──────────────────────────────────┘
```

### Complete Example

```rust
use vil_server::prelude::*;
use vil_server_macros::vil_wasm;

// Multiple functions can share one WASM module
#[vil_wasm(module = "pricing")]
fn calculate_price(base_cents: i32, qty: i32) -> i32 {
    let discount = if qty >= 100 { 20 } else if qty >= 50 { 10 } else { 0 };
    (base_cents as i64 * qty as i64 * (100 - discount) / 100) as i32
}

#[vil_wasm(module = "pricing")]
fn calculate_tax(price_cents: i32, tax_bps: i32) -> i32 {
    (price_cents as i64 * tax_bps as i64 / 10000) as i32
}

#[vil_wasm(module = "pricing", pool_size = 8, timeout_ms = 3000)]
fn apply_discount(price_cents: i32, discount_pct: i32) -> i32 {
    (price_cents as i64 * (100 - discount_pct as i64) / 100) as i32
}

// In handler — called like normal functions
async fn process(ctx: ServiceCtx, body: ShmSlice)
    -> HandlerResult<VilResponse<Result>>
{
    let req: Request = body.json()?;
    let price = calculate_price(req.base, req.qty);   // → WASM
    let tax = calculate_tax(price, 1100);              // → WASM
    Ok(VilResponse::ok(Result { total: price + tax }))
}

// Main — zero plumbing
#[tokio::main]
async fn main() {
    VilApp::new("my-app").port(8080)
        .service(ServiceProcess::new("svc")
            .endpoint(Method::POST, "/process", post(process)))
        .run().await;
}
```

### Native Fallback

If the `.wasm` file is not built/available, `#[vil_wasm]` automatically
falls back to running the Rust function body natively:

```
[VIL] WASM module 'pricing' not available — running pricing.calculate_price()
      as native Rust fallback. Build WASM modules or enable --features wasm
      for sandboxed execution.
```

This means your code **always works** — with or without WASM compilation.
Development: runs native. Production: compile `.wasm` for isolation.

### WASM Module Resolution

The bridge searches these paths (in order):
1. `$VIL_WASM_DIR/{module}.wasm` (env var override)
2. `wasm-modules/out/{module}.wasm`
3. `wasm-modules/{module}.wasm`
4. `{module}.wasm`

---

## 3. `#[vil_sidecar]` — Process-Isolated Execution

### When to Use

- Polyglot: Python ML models, Go microservices, Java legacy code
- Process isolation: crash in sidecar doesn't crash host
- Independent deployment: update sidecar without recompiling host
- **NOT for pure Rust** — native async is always more efficient

### How It Works

```
Developer writes:              VIL generates:
┌──────────────────────┐      ┌──────────────────────────────────┐
│ #[vil_sidecar]       │      │ 1. Async bridge fn               │
│ async fn score(data) │ ──→  │ 2. Auto-register in SidecarReg   │
│   { logic }          │      │ 3. dispatcher::invoke() via SHM  │
└──────────────────────┘      │ 4. Deserialization of response   │
                              │ 5. Metadata for introspection    │
                              └──────────────────────────────────┘
```

### Complete Example

```rust
use vil_server::prelude::*;
use vil_server_macros::vil_sidecar;

#[derive(Clone, Debug, Deserialize, Serialize)]
struct CreditScore {
    score: f64,
    risk_class: String,
    factors: Vec<String>,
}

// In Rust examples: body is the actual logic (runs as demo).
// In production with Python: source = "sidecars/scorer.py"
#[vil_sidecar(target = "credit-scorer")]
async fn score_credit(data: &[u8]) -> CreditScore {
    let req: serde_json::Value = serde_json::from_slice(data).unwrap_or_default();
    let income = req["income"].as_i64().unwrap_or(0);
    let debt = req["existing_debt"].as_i64().unwrap_or(0);

    let dti = if income > 0 { debt as f64 / income as f64 } else { 1.0 };
    let score = ((1.0 - dti) * 100.0).max(0.0).min(100.0);
    let risk_class = if score >= 80.0 { "A" } else if score >= 60.0 { "B" } else { "C" };

    CreditScore {
        score,
        risk_class: risk_class.into(),
        factors: vec![format!("dti:{:.0}%", dti * 100.0)],
    }
}

// In handler — called like a normal async function
async fn assess(ctx: ServiceCtx, body: ShmSlice)
    -> HandlerResult<VilResponse<CreditScore>>
{
    let result = score_credit(body.as_bytes()).await;  // → Sidecar
    Ok(VilResponse::ok(result))
}
```

### Sidecar Communication Protocol

```
Rust Host                         Sidecar Process
┌──────────┐                     ┌──────────────┐
│ Handler  │                     │ Python/Go    │
│          │ ── invoke() ──────→ │              │
│          │    (SHM + UDS)      │ execute fn   │
│          │ ←── response ────── │              │
│          │    (SHM + UDS)      │              │
└──────────┘                     └──────────────┘
     │                                 │
     └────── /dev/shm/vil_sc_* ───────┘
              (zero-copy data)
```

- **Transport:** Unix Domain Socket (~48 bytes per message — descriptors only)
- **Data Plane:** `/dev/shm/vil_sc_{name}` (zero-copy via mmap)
- **Protocol:** Length-prefixed JSON

---

## 4. Mixed Execution Pattern

The showcase pattern: one handler uses all three execution modes.
See [example 023](../../examples/023-basic-hybrid-wasm-sidecar/).

```rust
#[vil_wasm(module = "pricing")]
fn calculate_price(base: i32, qty: i32) -> i32 { /* ... */ }

#[vil_sidecar(target = "fraud")]
async fn check_fraud(data: &[u8]) -> FraudResult { /* ... */ }

async fn process_order(ctx: ServiceCtx, body: ShmSlice)
    -> HandlerResult<VilResponse<OrderResult>>
{
    let order: OrderRequest = body.json()?;

    // Native — validation (<100μs)
    if order.qty <= 0 { return Err(VilError::bad_request("qty > 0")); }

    // WASM — pricing rules (~1-5μs, sandboxed)
    let price = calculate_price(order.base, order.qty);

    // Sidecar — fraud scoring (~12μs, process-isolated)
    let fraud = check_fraud(&serde_json::to_vec(&order)?).await;

    // Native — finalization
    Ok(VilResponse::ok(OrderResult { price, fraud, .. }))
}
```

**NOTE:** This example is pure Rust. In production, native Rust does NOT
need WASM or sidecar — those add overhead. This demonstrates the PATTERN
for when you need isolation (WASM) or polyglot integration (sidecar).

---

## 5. Activity-Level Design

WASM and sidecar operate at **activity level**, not endpoint level.
VIL handles the endpoint (HTTP, routing, SHM). Custom code runs as an
activity within the endpoint handler.

```
Client → VIL (endpoint) → Handler {
                             ├── native validation (activity 1)
                             ├── WASM pricing     (activity 2)
                             ├── sidecar fraud     (activity 3)
                             └── native finalize   (activity 4)
                           } → VIL (response)
```

Data stays in SHM throughout. Each activity reads/writes the same
SHM region. Tri-Lane token transfers between activities.

This is NOT a router pattern — VIL is not just forwarding requests
to WASM/sidecar. VIL **orchestrates** the activity chain within one
endpoint, managing the SHM lifecycle and token flow.

---

## 6. Performance

| Mode | req/s | P50 | P99 | Isolation |
|------|-------|-----|-----|-----------|
| Native | ~40,000 | 0.5ms | 25ms | None |
| WASM | ~35,000 | 0.6ms | 28ms | Memory sandbox |
| Sidecar | ~8,000 | 2ms | 15ms | Full process |

### Decision Guide

| Scenario | Mode | Why |
|----------|------|-----|
| Maximum performance, same team, Rust | **Native** | Zero overhead, compiled in |
| Business rules that change weekly | **WASM** | Hot-deploy without restart |
| Untrusted/third-party code | **WASM** | Memory sandbox isolation |
| Python ML model (XGBoost, sklearn) | **Sidecar** | Full Python runtime needed |
| Go/Java legacy microservice | **Sidecar** | Polyglot, no rewrite needed |
| Pure Rust, no isolation needed | **Native** | WASM/sidecar add unnecessary overhead |

### SDK (Non-Rust) Developer Pattern

| Language | Pipeline Definition | Custom Business Logic |
|----------|--------------------|-----------------------|
| **Python** | Transpile SDK → YAML → binary | **Sidecar** (UDS + SHM) |
| **Go** | Transpile SDK → YAML → binary | **Sidecar** (UDS + SHM) |
| **Java** | Transpile SDK → YAML → binary | **Sidecar** (UDS + SHM) |
| **TypeScript** | Transpile SDK → YAML → binary | **WASM** or **Sidecar** |

---

## 7. Related Examples

| Example | Pattern | Features |
|---------|---------|----------|
| [021 WASM FaaS](../../examples/021-basic-wasm-faas/) | WASM only | Real wasmtime, pricing/validation/transform |
| [022 Sidecar Python](../../examples/022-basic-sidecar-python/) | Sidecar only | `#[vil_sidecar]` credit scoring |
| [023 Hybrid](../../examples/023-basic-hybrid-wasm-sidecar/) | All 3 modes | `#[vil_wasm]` + `#[vil_sidecar]` + native |

---

*Previous: [Part 10 — Observer Dashboard](./010-VIL-Developer_Guide-Observer-Dashboard.md)*
