# Phase 4 — Q2 2027: SDK & Platform

> **⚠ MANDATORY: Read [COMPLIANCE.md](./COMPLIANCE.md) before implementing any crate in this phase.**
> Every crate must pass the full compliance checklist (P1–P10, testing, docs, pre-merge review).
> Non-compliant crates will be rejected regardless of functionality.

## Objective

Expand VIL's language reach with 4 new SDK languages and build platform services (Cloud, Marketplace, Playground) that make VIL accessible to a wider developer audience.

---

## Part A: SDK Language Expansion

### Architecture Recap

VIL's Transpile SDK works as follows:

```
Developer writes SDK code (Python/Go/Java/TS/...)
    │
    ▼
vil compile --from <lang> --input <source>
    │
    ▼
SDK source → app.vil.yaml manifest
    │
    ▼
YAML → Rust codegen → cargo build
    │
    ▼
Single native binary (no runtime dependency)
```

New languages follow the same pattern. The key component per language:
1. **SDK library** — fluent API for defining pipelines in that language
2. **Transpiler** — converts SDK code → `app.vil.yaml`
3. **`vil init --lang <lang>`** — project scaffolding
4. **Template support** — all 8 existing templates must work

---

### A1. C# / .NET (`vil init --lang csharp`)

**Priority**: High (enterprise .NET ecosystem is massive)

**SDK Design**:
```csharp
// app.vil.cs
using Vil.Sdk;

var pipeline = new VilPipeline("credit-gateway")
    .Port(3080)
    .Source(new HttpSource("ingest")
        .Method(HttpMethod.Post)
        .Path("/api/credits"))
    .Transform("enrich", (ctx, record) => {
        record["risk"] = record["score"].AsInt() < 500 ? "high" : "low";
        return record;
    })
    .Sink(new HttpSink("upstream")
        .Url("http://core-banking:8080/api/credits"))
    .Build();

VilRunner.Run(pipeline);
```

**Implementation Plan**:
```
crates/vil_sdk_csharp/
├── src/
│   ├── lib.rs
│   ├── parser.rs       — parse .cs file → extract pipeline definition
│   ├── transpiler.rs   — C# AST → app.vil.yaml
│   └── error.rs
sdk/csharp/
├── Vil.Sdk/
│   ├── VilPipeline.cs
│   ├── HttpSource.cs
│   ├── HttpSink.cs
│   ├── VilRunner.cs
│   └── Vil.Sdk.csproj
└── templates/           — per-template .cs scaffolds
```

**Transpiler approach**:
- Parse C# using regex/AST extraction (not full Roslyn — just the fluent builder pattern)
- Extract: pipeline name, port, sources, sinks, transforms
- Emit `app.vil.yaml` identical to what Rust/Python/Go/Java/TS produce
- From YAML onward, existing codegen pipeline handles everything

**Testing**:
- `vil init --lang csharp --template <all 8>` must produce valid YAML
- `vil compile` on each must produce identical binary to Rust equivalent
- SDK NuGet package for local development experience

**Estimated effort**: 5-7 days

---

### A2. Kotlin (`vil init --lang kotlin`)

**Priority**: Medium-High (Android + JVM enterprise)

**SDK Design**:
```kotlin
// app.vil.kt
import id.vastar.vil.sdk.*

fun main() {
    vilPipeline("credit-gateway") {
        port(3080)
        source(httpSource("ingest") {
            method(HttpMethod.POST)
            path("/api/credits")
        })
        transform("enrich") { ctx, record ->
            record["risk"] = if (record.getInt("score") < 500) "high" else "low"
            record
        }
        sink(httpSink("upstream") {
            url("http://core-banking:8080/api/credits")
        })
    }.run()
}
```

**Implementation Plan**:
```
crates/vil_sdk_kotlin/
├── src/
│   ├── lib.rs
│   ├── parser.rs       — parse .kt → pipeline extraction
│   ├── transpiler.rs   — Kotlin DSL → app.vil.yaml
│   └── error.rs
sdk/kotlin/
├── vil-sdk/
│   ├── src/main/kotlin/id/vastar/vil/sdk/
│   │   ├── VilPipeline.kt
│   │   ├── HttpSource.kt
│   │   ├── HttpSink.kt
│   │   └── VilRunner.kt
│   └── build.gradle.kts
└── templates/
```

**Estimated effort**: 4-5 days (similar to Java, Kotlin DSL is natural fit)

---

### A3. Swift (`vil init --lang swift`)

**Priority**: Medium (iOS/macOS edge deployment)

**SDK Design**:
```swift
// app.vil.swift
import VilSDK

let pipeline = VilPipeline("credit-gateway")
    .port(3080)
    .source(HttpSource("ingest")
        .method(.post)
        .path("/api/credits"))
    .transform("enrich") { ctx, record in
        record["risk"] = record["score"].intValue < 500 ? "high" : "low"
        return record
    }
    .sink(HttpSink("upstream")
        .url("http://core-banking:8080/api/credits"))

VilRunner.run(pipeline)
```

**Implementation Plan**:
```
crates/vil_sdk_swift/
├── src/
│   ├── lib.rs
│   ├── parser.rs       — parse .swift → pipeline extraction
│   ├── transpiler.rs   — Swift builder → app.vil.yaml
│   └── error.rs
sdk/swift/
├── Sources/VilSDK/
│   ├── VilPipeline.swift
│   ├── HttpSource.swift
│   ├── HttpSink.swift
│   └── VilRunner.swift
├── Package.swift
└── templates/
```

**Estimated effort**: 4-5 days

---

### A4. Zig (`vil init --lang zig`)

**Priority**: Low-Medium (systems programming niche, but growing fast)

**SDK Design**:
```zig
// app.vil.zig
const vil = @import("vil-sdk");

pub fn main() !void {
    var pipeline = vil.Pipeline.init("credit-gateway")
        .port(3080)
        .source(vil.HttpSource.init("ingest")
            .method(.post)
            .path("/api/credits"))
        .sink(vil.HttpSink.init("upstream")
            .url("http://core-banking:8080/api/credits"));

    try vil.run(pipeline);
}
```

**Implementation Plan**:
```
crates/vil_sdk_zig/
├── src/
│   ├── lib.rs
│   ├── parser.rs       — parse .zig → pipeline extraction
│   ├── transpiler.rs   — Zig builder → app.vil.yaml
│   └── error.rs
sdk/zig/
├── src/
│   ├── vil_sdk.zig
│   ├── pipeline.zig
│   ├── http_source.zig
│   └── http_sink.zig
├── build.zig
└── templates/
```

**Estimated effort**: 4-5 days

---

### SDK Compliance Requirements

All new SDK languages must:

| Requirement | Check |
|-------------|-------|
| `vil init --lang <lang>` generates valid project | ☐ |
| All 8 templates produce valid `app.vil.yaml` | ☐ |
| Generated YAML is **identical** to Rust-native YAML for same template | ☐ |
| `vil compile --from <lang>` produces working binary | ☐ |
| SDK library publishable to language package registry (NuGet, Maven Central, Swift PM, etc.) | ☐ |
| README with getting-started guide per language | ☐ |
| Integration test: init → compile → run → verify output | ☐ |

---

## Part B: Platform Services

### B1. crates.io Publish

**Priority**: High (prerequisite for ecosystem growth)

**Scope**:
- Fix all 100 publishable crates' `Cargo.toml` metadata
- Add `version = "0.1.0"` to all path dependencies
- Determine publish order (dependency graph bottom-up)
- Automate publish script (`scripts/publish-all.sh`)
- Reserve `vil_*` crate names

**Implementation Plan**:

1. **Metadata fix** (batch script):
   ```toml
   # Add to [workspace.package]
   repository = "https://github.com/OceanOS-id/VIL"
   homepage = "https://vastar.id/products/vil"
   documentation = "https://vastar.id/docs/vil"
   ```
   Then per-crate: `repository.workspace = true`, `homepage.workspace = true`, etc.

2. **Path dependency versioning**:
   ```toml
   # Before
   vil_types = { path = "../vil_types" }
   # After
   vil_types = { version = "0.1.0", path = "../vil_types" }
   ```

3. **Publish order** — generate from dependency graph:
   ```
   Layer 1: vil_types, vil_json
   Layer 2: vil_shm, vil_macros
   Layer 3: vil_queue, vil_registry
   Layer 4: vil_rt, vil_obs, vil_net
   Layer 5: vil_ir, vil_validate, vil_codegen_rust, vil_codegen_c
   ...
   Layer N: vil_cli, vil_sdk (last)
   ```

4. **Per-crate README** — auto-generate from Cargo.toml description + standard template

**Estimated effort**: 3-5 days (mostly scripting)

---

### B2. VIL Cloud — Managed Deployment (SaaS)

**Priority**: Medium (revenue potential)

**Scope**:
- Deploy VIL pipelines with `vil deploy --target cloud`
- Container orchestration (Kubernetes-based)
- Auto-scaling based on Tri-Lane metrics
- Dashboard: pipeline status, throughput, latency
- Multi-tenant namespace isolation
- Secrets management (vault integration)
- Log aggregation + VIL obs metrics export

**Architecture**:
```
Developer Machine                    VIL Cloud
┌─────────────┐                     ┌──────────────────────┐
│ vil deploy   │───── gRPC ────────►│ Deploy Service       │
│ --target     │                    │   ├─ Build Pipeline   │
│   cloud      │                    │   ├─ Push Image       │
└─────────────┘                    │   └─ K8s Apply        │
                                   ├──────────────────────┤
                                   │ Runtime               │
                                   │   ├─ Namespace/Tenant │
                                   │   ├─ Auto-scale       │
                                   │   └─ vil_obs export   │
                                   ├──────────────────────┤
                                   │ Dashboard             │
                                   │   ├─ Pipeline status  │
                                   │   ├─ Metrics/logs     │
                                   │   └─ Secrets mgmt     │
                                   └──────────────────────┘
```

**This is a large initiative** — break into sub-phases:
- B2a: CLI `vil deploy` + container build (Q2 2027)
- B2b: K8s runtime + auto-scale (Q3 2027)
- B2c: Dashboard + multi-tenant (Q4 2027)

**Estimated effort**: 3-6 months (ongoing)

---

### B3. VIL Marketplace — Community Connectors & Templates

**Priority**: Medium

**Scope**:
- Web portal at `marketplace.vastar.id` (or section of `vastar.id`)
- Community can publish: connectors (crates), templates, example pipelines
- Review + approval workflow
- Search + filter by category
- `vil install <package>` CLI command
- Rating + download counts

**Implementation Plan**:
- Registry backend (simple REST API + PostgreSQL)
- Frontend (React, consistent with vastar.id website)
- CLI integration in `vil_cli`
- Publisher authentication (GitHub OAuth)

**Estimated effort**: 2-3 months

---

### B4. VIL Playground — Browser WASM Sandbox

**Priority**: Medium (developer onboarding)

**Scope**:
- Browser-based editor (Monaco)
- VIL compiler compiled to WASM (runs in browser)
- Pre-loaded templates
- Live output preview (simulated pipeline)
- Share via URL (encoded config)
- No server-side execution (pure client-side)

**Architecture**:
```
Browser
┌────────────────────────────────────┐
│ Monaco Editor (YAML + Rust)        │
│         │                          │
│         ▼                          │
│ vil_compiler.wasm                  │
│   ├─ Parse YAML                    │
│   ├─ Validate                      │
│   └─ Show IR / errors              │
│         │                          │
│         ▼                          │
│ Simulated Pipeline Output          │
│ (mock HTTP source/sink)            │
└────────────────────────────────────┘
```

**Key constraint**: Compiler only, no actual network I/O. Pipeline "runs" against mock data.

**Estimated effort**: 3-4 weeks

---

## Development Order

1. **crates.io publish** — unblocks everything (week 1-2)
2. **C# SDK** — highest enterprise demand (week 3-4)
3. **Kotlin SDK** — JVM + Android (week 4-5)
4. **Swift SDK** — iOS/macOS (week 5-6)
5. **Zig SDK** — systems niche (week 6-7)
6. **VIL Playground** — marketing + onboarding (week 7-10)
7. **VIL Cloud** — start B2a (week 10+)
8. **VIL Marketplace** — after Cloud MVP (Q3 2027+)

---

## Milestone Checklist

- [ ] crates.io — all 100 crates published with correct metadata
- [ ] C# SDK — init + compile + all 8 templates + NuGet package
- [ ] Kotlin SDK — init + compile + all 8 templates + Maven Central
- [ ] Swift SDK — init + compile + all 8 templates + Swift PM
- [ ] Zig SDK — init + compile + all 8 templates + build.zig
- [ ] VIL Playground — browser editor + WASM compiler + mock output
- [ ] VIL Cloud — `vil deploy` MVP with container build
- [ ] VIL Marketplace — portal + `vil install` CLI
- [ ] COMPLIANCE.md checklist passed for all SDK crates
- [ ] Website docs updated for all new languages
