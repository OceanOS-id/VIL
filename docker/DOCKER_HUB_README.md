# VIL Provisionable Server

**Vastar Intermediate Language — provisionable workflow runtime.** Starts empty, accepts workflow uploads at runtime, hot-reloads routes in ~200ms. Mixes native Rust, WASM (4 languages), and sidecar (9 languages) handlers per activity.

> 🔶 **IMPORTANT — LICENSING.** This image is distributed under the **Vastar Source Available License (VSAL)**, **NOT** Apache 2.0 / MIT. Internal business use is free. **Operating this image as a multi-tenant Workflow-as-a-Service (WaaS) requires a separate commercial agreement with Vastar** (legal@midsolution.id). See the [Licensing](#licensing) section below before deploying.

---

## Quick Start — AI Gateway in 60 Seconds

Run the VIL provisionable server, upload a pre-built AI-gateway workflow, hit it with `curl`, benchmark it. No `git clone`, no `cargo build` — just 3 curl commands once the server is up.

```bash
# 0. (prerequisite, one-time) Install the upstream LLM simulator binary from crates.io
cargo install ai-endpoint-simulator
ai-endpoint-simulator &                                   # listens on :4545

# 1. Start the provisionable VIL server (empty — no workflows yet)
docker run -d --network host --name vil vilfounder/vil:0.4.0

# 2. Download the pre-built AI-gateway sample bundle
curl -sSL https://github.com/OceanOS-id/VIL/releases/download/v0.4.0/sample-ai-gateway.tar.gz \
  | tar xz
cd sample

# 3. Upload the workflow YAML (no WASM needed — pure Connector workflow)
./curl-upload.sh

# 4. Hit the endpoint — streams SSE through vil-server to the simulator
curl -N -X POST http://localhost:3080/trigger \
  -H 'Content-Type: application/json' \
  -d '{"prompt":"hello"}'

# 5. Benchmark with vastar (install: cargo install vastar-bench, or curl the installer)
vastar -c 200 -z 5s -m POST \
  -H 'Content-Type: application/json' \
  -d '{"prompt":"bench"}' \
  http://localhost:3080/trigger
```

What this sample demonstrates:
- **Provisionable mode** — workflow uploaded at runtime via the admin API, hot-reload in ~200ms
- **Tri-Lane SSE proxy** — `activity_type: Connector` forwards to an upstream LLM endpoint with streaming
- **Zero authoring** — user brings only YAML; no `.wasm`, no sidecar for this sample

**Want WASM or sidecar samples instead?** Use `sample-003-credit-scoring.tar.gz` (WASM), `sample-034-blocking-task.tar.gz` (Python sidecar), etc. — see the [releases page](https://github.com/OceanOS-id/VIL/releases).

## Tags

| Tag | Contents |
|-----|----------|
| `0.4.0`, `0.4`, `latest` | Current stable |
| `0.4.0-amd64` / `0.4.0-arm64` | Explicit arch pins |

Multi-arch: `linux/amd64`, `linux/arm64`.

## Configuration

All configuration is via environment variables:

| Env | Default | Purpose |
|-----|---------|---------|
| `PORT` | `3080` | HTTP listen port |
| `ADMIN_KEY` | *(empty — OPEN)* | API key gating `/api/admin/*`. **Set this if the server is reachable from outside localhost.** |
| `WORKFLOWS_DIR` | `/var/lib/vil/workflows` | Workflow YAML persistence |
| `VIL_PLUGIN_DIR` | `/var/lib/vil/plugins` | `.so` NativeCode handlers |
| `VIL_WASM_DIR` | `/var/lib/vil/modules` | `.wasm` modules |
| `VIL_LOG` | `info` | Log level (`trace`, `debug`, `info`, `warn`, `error`) |

## Volumes (Persistence)

```bash
docker run -d -p 3080:3080 \
  -e ADMIN_KEY=your-secret-token \
  -v vil-workflows:/var/lib/vil/workflows \
  -v vil-plugins:/var/lib/vil/plugins \
  -v vil-modules:/var/lib/vil/modules \
  --name vil \
  vilfounder/vil:0.4.0
```

Provisioned workflows and handler artifacts survive container restarts when these volumes are mounted.

## Admin API Endpoints

All under `/api/admin/`, gated by `ADMIN_KEY` if set:

| Method | Path | Purpose |
|--------|------|---------|
| `POST` | `/upload` | Upload workflow YAML (auto-provisions referenced handlers) |
| `POST` | `/upload/plugin` | Upload `.so` NativeCode handler |
| `POST` | `/upload/wasm` | Upload `.wasm` module |
| `GET`  | `/handlers` | List registered handlers |
| `GET`  | `/workflows` | List registered workflows |
| `GET`  | `/workflow/status` | Inspect workflow status |
| `POST` | `/workflow/activate` | Activate a staged workflow |
| `POST` | `/workflow/deactivate` | Deactivate without deleting |
| `DELETE` | `/workflow` | Remove a workflow |
| `POST` | `/reload` | Force re-scan of workflow directory |
| `GET`  | `/health` | Admin API health check |

## What Runs Inside

- **Base**: `debian:bookworm-slim`
- **Runtime**: `vil-server` binary (from `vil_server_provision` crate)
- **User**: non-root (`vil:vil`, uid 999)
- **Exposed**: port 3080
- **Image size**: ~100 MB

## Architecture

```
┌─────────────────────────────────────────────────────────┐
│  vil-server  (this container)                           │
│  Admin API — /api/admin/* (gated by ADMIN_KEY)         │
│     ↓                                                   │
│  Registry → VWFD Compiler → Route Installer            │
│     ↓                                                   │
│  Hot Reload — existing traffic continues unaffected   │
│                                                         │
│  Handler Types (mixable per-activity):                 │
│    • NativeCode — .so plugin (Rust)                    │
│    • Function   — .wasm (Rust, AssemblyScript, C, Java)│
│    • Sidecar    — subprocess command referenced by YAML│
│                   (Python, Node.js, Java, Go, C#, PHP, │
│                    Lua, Ruby, R)                       │
└─────────────────────────────────────────────────────────┘
```

## Licensing

> ⚠️ **This image is NOT open source under the OSI definition.** It is distributed under the **Vastar Source Available License (VSAL)** — [full text on GitHub](https://github.com/OceanOS-id/VIL/blob/main/LICENSE-VSAL).

### ✓ What's FREE and Permitted

- **Pull, run, modify, redistribute** this image for:
  - Internal company use (automation, data pipelines, internal APIs)
  - Self-hosted deployment of your own workflows
  - Embedding VIL workflows as a component of a **Significant Business Process** (credit scoring, IoT, KYC, banking, telehealth, insurance, HR, manufacturing MES, e-government, LMS, e-commerce fulfillment, etc.)
  - Distributing a product that includes this image to clients who self-host it on their own infrastructure
- See the [Significant Business Process Exception](https://github.com/OceanOS-id/VIL/blob/main/LICENSING.md#34-significant-business-process-exception) for the full test and example matrix.

### ✗ What REQUIRES a Commercial Agreement

**Operating this image as a Workflow-as-a-Service (WaaS)** — that is, any service whose primary product is letting third parties upload, deploy, execute, or manage their own workflow definitions on infrastructure you operate. Including:

- Hosted n8n-alternative / Kestra-alternative / Temporal-alternative built on VIL
- "Managed VIL Workflows" cloud product
- White-labeled VWFD hosting service
- Multi-tenant Provisionable-Mode hosting for external customers
- **Translation layers** that accept n8n / Kestra / Airflow / Temporal workflow formats, convert them to VWFD, and host execution on your infrastructure

The restriction applies regardless of whether the service is free or paid. Contact **legal@midsolution.id** for commercial WaaS licensing.

### Why This License?

VSAL exists specifically to preserve Workflow-as-a-Service reselling as Vastar's commercial moat, while keeping the runtime free for the 99% of use cases where VIL is a component of someone's own business. The same model as MongoDB SSPL, Elastic License v2, and BSL — source is available, internal use is free, the licensor (Vastar) keeps exclusive commercial hosting rights.

### In a Hurry? The 2-Sentence Version

> If you're running this image to host **your own** workflows — however you use them, including as a product feature for your customers — you are fully permitted. If you're running this image to let your customers **upload their own workflows** and have you host them, contact Vastar first.

---

## Links

- **Source (VSAL)**: [github.com/OceanOS-id/VIL](https://github.com/OceanOS-id/VIL)
- **Product page**: [vastar.id/products/vil](https://vastar.id/products/vil)
- **Documentation**: [vastar.id/docs/vil](https://vastar.id/docs/vil)
- **Licensing guide**: [LICENSING.md](https://github.com/OceanOS-id/VIL/blob/main/LICENSING.md)
- **Full VSAL text**: [LICENSE-VSAL](https://github.com/OceanOS-id/VIL/blob/main/LICENSE-VSAL)
- **Commercial inquiries**: legal@midsolution.id
