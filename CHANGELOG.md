# Changelog

All notable changes to VIL will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

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
