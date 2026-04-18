# Changelog

All notable changes to VIL will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.4.0] - 2026-04-18

This release ships a major licensing restructure, a pre-built Docker image, runtime workflow provisioning, and a verified polyglot language matrix. Upgrade notes below.

### Licensing — breaking for WaaS operators only

- **Added**: Vastar Source Available License (VSAL) — formal license text at [`LICENSE-VSAL`](LICENSE-VSAL). A source-available license in the Sustainable Use family (n8n SUL, Elastic 2.0, BSL). Internal business use remains free; commodity Workflow-as-a-Service hosting requires a separate commercial agreement.
- **Added**: [`LICENSING.md`](LICENSING.md) — ecosystem licensing guide covering two-tier model, §3.3.1 anti-translation clause, §3.7.5 Licensor Reserved Rights, §3.8 example matrix (~24 scenarios), §3.9 SUL family comparison, §6 Cloud Services commercial moat.
- **Changed**: 7 crates moved from Apache/MIT to VSAL — `vil_vwfd`, `vil_vwfd_macros`, `vil_server_provision`, `vil_cli`, `vil_cli_server`, `vil_workflow_v2`, `vil_operator`. Each has `publish = false` — install from GitHub or the Docker image.
- **Unchanged**: ~165 library crates remain Apache 2.0 / MIT (dual), including `vil_server` (umbrella), `vil_server_core`, all connectors, triggers, FaaS, AI/LLM plugins, SDK, observability.
- **Significant Business Process Exception** (§3.6): if VIL workflows are a component of a product with substantial domain value (credit scoring, IoT, banking, KYC, telehealth, HR, manufacturing MES, e-government, LMS, insurance, e-commerce fulfillment, etc.), usage is permitted without any commercial agreement.

### Added

- **`vil-server` Docker image** — pre-built provisionable server published at [`vilfounder/vil:0.4.0`](https://hub.docker.com/r/vilfounder/vil). Multi-arch (linux/amd64, linux/arm64). Two variants:
  - `0.4.0` / `0.4` / `latest` — debian-slim (~180 MB, shell available for debugging)
  - `0.4.0-slim` / `0.4-slim` / `slim` — distroless (~50 MB, smallest pull, production-ideal)
  OCI labels include explicit VSAL metadata + WaaS restriction warning.
- **`.provision(true)` API** — any `VilApp` / `VwfdApp` can mount the admin API at `/api/admin/*` with a single builder flag. Supports runtime workflow upload, hot-reload in ~200ms, optional `.provision_key()` auth. See [`public/docs/vil/guides/provisionable-workflow.md`](public/docs/vil/guides/provisionable-workflow.md).
- **Sample workflow bundles** — pre-built `.tar.gz` artifacts uploadable via 3 curl commands (no cargo, no git clone). AI gateway sample published at `releases/sample-ai-gateway.tar.gz`. GitHub Actions workflow at [`.github/workflows/release-samples.yml`](.github/workflows/release-samples.yml) auto-uploads on tag push.
- **Polyglot language matrix verified** — 12 languages now production-tested:
  - WASM (4): Rust, AssemblyScript, C, Java (TeaVM)
  - Sidecar (9): Python, Node.js, Java, Go, C#, PHP, Lua, Ruby, R
- **Docker tooling**:
  - `Dockerfile` (debian-slim) + `Dockerfile.slim` (distroless) at repo root
  - `docker-compose.yml` — vil + simulators + 12-service infra profile
  - `scripts/docker-publish.sh` — multi-arch buildx wrapper
  - `scripts/package-samples.sh` — registry-driven sample bundler
  - `docker/DOCKER_HUB_README.md` — Docker Hub long description with VSAL + WaaS prominent
- VWFD validator rejects YAML where sidecar/wasm activities reference `vastar.db.*` / `vastar.mq.*` / `vastar.trigger.*` — enforces "connectors + triggers are Rust-only" discipline.

### Changed

- **Workspace version**: `0.3.0` → `0.4.0` across root + ~180 `Cargo.toml` files. All VIL-internal deps updated to `version = "0.4"`.
- **`vil_cli` moved to VSAL** — the `vil` binary dispatcher is part of the VWFD dev loop. Install from source: `cargo install --git https://github.com/OceanOS-id/VIL --tag v0.4.0 vil_cli`.
- **`LICENSE-VSAL` preamble** — enumerates all 7 VSAL crates.
- **`README.md`**:
  - Added two-tier license section with ~165 Apache/MIT + 7 VSAL split
  - Significant Business Process Exception block with ~15 permitted product examples
  - Quick Start updated for `cargo install --git` (VSAL) vs `cargo add <crate>` (Apache/MIT)
  - Hero badges: replaced "15 Languages" claim with "4 WASM + 9 Sidecar Langs" + "Provisionable" pill

### Fixed

- **`examples/010-basic-websocket-chat/vwfd/test_vwfd.sh`**: removed stray `fi` on line 84 that aborted the script before `print_summary`.
- **`examples/205-llm-chunked-summarizer`**: chat-source transform made dual-purpose — previously returned `None` on response chunks (json_tap content) so SSE body arrived empty; now pass-throughs non-request payloads. POST `/summarize` returns 1696+ chars (was 0).
- **Bulk sed collateral** from the 0.3 → 0.4 bump: reverted over-matched third-party dep versions in Cargo.toml files (`tracing-subscriber`, `futures`, `sonic-rs` back to `0.3`).

### Removed

- Nothing removed in v0.4.0.

### Migration from 0.3.x

1. **If you run VIL workflows internally** (your own workflow definitions, any product feature): no action needed. Your use remains free under VSAL. Update to `version = "0.4"` in your `Cargo.toml` or pull the new Docker image.
2. **If you host third-party workflows as a primary product (WaaS)**: contact `legal@midsolution.id` for a commercial agreement before deploying v0.4.0.
3. **If you depend on `vil_cli` via `cargo install`**: switch to `cargo install --git https://github.com/OceanOS-id/VIL --tag v0.4.0 vil_cli` (it's no longer on crates.io).
4. **If you depend on `vil_vwfd` from crates.io**: switch to `vil_vwfd = { git = "https://github.com/OceanOS-id/VIL", tag = "v0.4.0" }` in your `Cargo.toml`.
5. **All other library crates** remain on crates.io with unchanged Apache/MIT terms — just bump their version constraint to `0.4`.

---

## [0.1.2] - 2026-03-29

### Security
- **secrets.rs**: Replace XOR cipher with real AES-256-GCM (`aes-gcm` crate). Key and nonce generation now use `getrandom` CSPRNG.
- **CSRF**: Replace `SystemTime`-based `rand_byte()` with `getrandom` CSPRNG.
- **CSRF**: Replace hand-rolled `constant_time_eq` with `subtle::ConstantTimeEq`.
- **API keys**: Store as SHA-256 hashes instead of plaintext. Validate with constant-time comparison.
- **BumpRegion**: Fix race condition — replace `fetch_add+store` with CAS loop to prevent overlapping allocations on wrap-around.
- **SHM bounds checks**: Use `checked_add()` in all bounds checks to prevent integer overflow bypass.
- **SSRF**: Add private IP blocklist to `vil_agent` HTTP fetch and `vil_crawler` crawl_url.
- **WASM sandbox**: Configure `wasmtime` with fuel metering (1B instruction default) and epoch interruption.
- **CORS**: Change default from permissive to disabled. Require explicit opt-in via `.cors_permissive()`.
- **HSTS**: Add `Strict-Transport-Security` header (1 year, includeSubDomains).
- **Rate limiter**: Add `cleanup_expired()` to prevent unbounded memory growth.
- **API key scopes**: Add `validate_scoped()` for scope enforcement.
- **SAFETY comments**: Add ~65 missing SAFETY comments to unsafe blocks across 6 core crates.
- Add `SECURITY.md` with vulnerability disclosure policy.

### Added
- GitHub Actions CI/CD: check, test, clippy, fmt, audit, deny.
- `deny.toml` for cargo-deny license and advisory enforcement.
- `rust-toolchain.toml` pinning Rust 1.93.1 for reproducible builds.
- `CONTRIBUTING.md` with development guidelines.
- `CODE_OF_CONDUCT.md` (Contributor Covenant 2.1).

### Fixed
- License SPDX: corrected from `"Apache-2.0"` to `"MIT OR Apache-2.0"` across workspace and all 61 crates with explicit license field.
- Removed 4 accidentally committed ELF binaries (~24MB) from git.
- `vil_observer`: set `default = []` features to fix publish order dependency on `vil_new_http`.

## [0.1.1] - 2026-03-28

### Added
- `vil_observer`: Embedded observer dashboard with live metrics, sparklines, routes, system info, SHM stats, and semantic events.
- `vil_server_core`: Observer wiring via `.observer(true)` builder method.
- Dual-architecture documentation (VilApp vs ShmToken).
- 4 new examples: observer dashboard (039), benchmarks (001b, 101b, 101c).

## [0.1.0] - 2026-03-27

### Added
- Initial release of 132 crates.
- Core runtime: `vil_types`, `vil_shm`, `vil_queue`, `vil_log`, `vil_registry`, `vil_rt`, `vil_engine`.
- Semantic compiler: `vil_ir`, `vil_validate`, `vil_macros`, `vil_codegen_rust`, `vil_codegen_c`.
- Server framework: `vil_server_core`, `vil_server_web`, `vil_server_mesh`, `vil_server_auth`, `vil_server_db`.
- SDK: `vil_sdk`, `vil_plugin_sdk`, `vil_new_http`.
- 10 database connectors, 7 message queue connectors, 3 storage connectors.
- 8 trigger crates, 4 protocol crates (SOAP, OPC-UA, Modbus, WebSocket).
- 40+ AI/LLM crates (RAG, agents, embeddings, guardrails).
- 84 examples across 8 tiers.
- 9 SDK transpile languages.
- 1,255+ tests.
