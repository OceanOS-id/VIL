# VIL Security Patch Report

**Date:** 2026-03-29
**Author:** Engineering Team + Automated Audit
**Scope:** Full codebase security hardening — 132 crates
**Commits:** `553b12e` → `0e68718` (9 commits)
**Crates Affected:** All 132 — version bump 0.1.0 → 0.1.1 (republish required)

---

## Executive Summary

Following an external SQA assessment and a comprehensive internal audit, **27 security findings** were identified and remediated across the VIL codebase. This report covers all patches applied in the 2026-03-29 security sprint.

**Breakdown by severity:**

| Severity | Found | Fixed | Remaining |
|----------|-------|-------|-----------|
| CRITICAL | 6 | 6 | 0 |
| HIGH | 9 | 9 | 0 |
| MEDIUM | 8 | 8 | 0 |
| Compliance | 4 | 4 | 0 |

---

## CRITICAL Fixes

### C-01: Fake AES-256-GCM Encryption in Secrets Provider
**File:** `vil_server_core/src/secrets.rs`
**Before:** XOR cipher labeled as `"ENC[AES256:...]"`. Key and nonce generated from `SystemTime::now().as_nanos()` (predictable).
**After:** Real AES-256-GCM via `aes-gcm` crate. Key and nonce generated via `getrandom` (OS CSPRNG).
**Impact:** All previously encrypted secrets must be re-encrypted. Old format is incompatible.
**Commit:** `6c601ea`

### C-02: Weak CSRF Token Generation
**File:** `vil_server_auth/src/csrf.rs`
**Before:** `rand_byte()` used `SystemTime::now().subsec_nanos()` — predictable, ~8 bits effective entropy per byte.
**After:** `getrandom::getrandom()` — OS-level CSPRNG, full 256-bit entropy for 32-byte tokens.
**Commit:** `553b12e`

### C-03: Hand-Rolled Constant-Time Comparison
**File:** `vil_server_auth/src/csrf.rs`
**Before:** Manual XOR-accumulate with early return on length mismatch (leaks length via timing).
**After:** `subtle::ConstantTimeEq` — audited, compiler-barrier-protected constant-time comparison.
**Commit:** `553b12e`

### C-04: Plaintext API Key Storage
**File:** `vil_server_auth/src/api_key.rs`
**Before:** API keys stored as plaintext `String` in `DashMap`. Validation via `DashMap::get()` (timing attack vector).
**After:** Keys stored as SHA-256 hashes (`sha2` crate). Validation iterates all entries with `subtle::ConstantTimeEq`.
**Commit:** `553b12e`

### C-05: BumpRegion Race Condition
**File:** `vil_shm/src/heap.rs`
**Before:** `fetch_add` + `store` on wrap-around — multiple threads could simultaneously reset cursor to 0, causing overlapping allocations and memory corruption.
**After:** Compare-and-swap (CAS) loop with `compare_exchange_weak`. Only one thread wins the wrap-around. Added `checked_add` to prevent integer overflow.
**Commit:** `63fdc29`

### C-06: License SPDX Mismatch
**File:** Root `Cargo.toml` + 61 crate `Cargo.toml` files
**Before:** `license = "Apache-2.0"` — but repository ships both MIT and Apache-2.0 license files.
**After:** `license = "MIT OR Apache-2.0"` — correct SPDX expression matching actual licensing.
**Commit:** `6c601ea`

---

## HIGH Fixes

### H-01: SSRF in Agent HTTP Fetch
**File:** `vil_agent/src/tools/http_fetch.rs`
**Before:** Fetches any URL without validation — allows access to `169.254.169.254` (cloud metadata), `10.0.0.0/8`, `127.0.0.1`.
**After:** `is_private_url()` blocklist checks: loopback, private RFC1918, link-local, CGN, cloud metadata hostnames, non-http schemes.
**Commit:** `63fdc29`

### H-02: SSRF in Crawler
**File:** `vil_crawler/src/crawler.rs`
**Before:** `crawl_url()` public method fetches any URL.
**After:** Same `is_private_url()` blocklist applied before every fetch.
**Commit:** `63fdc29`

### H-03: WASM Sandbox Without Resource Limits
**File:** `vil_capsule/src/host.rs`
**Before:** `wasmtime::Engine::default()` — no fuel limits, no memory caps. Infinite loop possible.
**After:** `wasmtime::Config` with `consume_fuel(true)`. Default 1 billion instruction budget per call. `epoch_interruption` support. All `Store` instances configured via `configure_store()`.
**Commit:** `63fdc29`

### H-04: Permissive CORS by Default
**File:** `vil_server_core/src/server.rs`
**Before:** `cors: true` default → `CorsLayer::permissive()` (any origin, any method).
**After:** `cors: false` default. New `.cors_permissive()` builder method for explicit opt-in.
**Commit:** `0f9040f`

### H-05: Integer Overflow in SHM Bounds Checks
**File:** `vil_shm/src/heap.rs`
**Before:** `if off + size > slot.buffer.len()` — wraps silently in release mode (`overflow-checks = false`).
**After:** `if off.checked_add(size).map_or(true, |end| end > slot.buffer.len())` — all 5 bounds checks fixed.
**Commit:** `0f9040f`

### H-06: Rate Limiter Unbounded Memory Growth
**File:** `vil_server_auth/src/rate_limit.rs`
**Before:** `DashMap<IpAddr, Bucket>` grows forever — no eviction logic.
**After:** Added `cleanup_expired()` method that retains only buckets active within 2x window. Added `bucket_count()` for monitoring.
**Commit:** `553b12e`

### H-07: Missing HSTS Header
**File:** `vil_server_auth/src/security.rs`
**Before:** 6 of 7 OWASP headers present. HSTS mentioned in doc comment but not implemented.
**After:** Added `Strict-Transport-Security: max-age=31536000; includeSubDomains`.
**Commit:** `553b12e`

### H-08: API Key Scopes Not Enforced
**File:** `vil_server_auth/src/api_key.rs`
**Before:** `scopes` field stored but `validate()` only checks `active` flag.
**After:** Added `validate_scoped(key, required_scope)` that checks scope membership.
**Commit:** `553b12e`

### H-09: Compiled Binaries in Git Repository
**Files:** 4 ELF executables in `examples/` (~24MB total)
**Before:** Committed to git — exposes internal binary structure, bloats repo.
**After:** Removed from tracking, added to `.gitignore`.
**Commit:** `6c601ea`

---

## MEDIUM Fixes

### M-01: Missing SAFETY Comments on Unsafe Code
**Files:** 6 core crates — `vil_log`, `vil_shm`, `vil_registry`, `vil_queue`, `vil_types`, `vil_rt`
**Before:** ~10% SAFETY comment coverage (24 of ~126 unsafe instances).
**After:** ~65 SAFETY comments added. Coverage improved to ~70%.
**Commit:** `553b12e`

### M-02: No CI/CD Pipeline
**Before:** Zero automated testing, linting, or security scanning.
**After:** GitHub Actions with 7 jobs: Check, Test, Clippy, Format, RustSec Audit, OSV Scanner, Cargo Deny.
**Commit:** `63fdc29`, `daef74b`, `0e68718`

### M-03: No Security Policy
**Before:** No `SECURITY.md` or vulnerability disclosure process.
**After:** `SECURITY.md` with disclosure instructions, SLA (48h ack, 7d critical fix), scope definition.
**Commit:** `63fdc29`

### M-04: No Dependency Policy
**Before:** No `cargo-deny` configuration. No license or advisory enforcement.
**After:** `deny.toml` with license allowlist (MIT, Apache-2.0, BSD, ISC), copyleft deny, unknown registry deny.
**Commit:** `63fdc29`

### M-05: No Reproducible Builds
**Before:** No `rust-toolchain.toml`. Builds depend on whatever Rust version is installed.
**After:** `rust-toolchain.toml` pinning Rust 1.93.1 + rustfmt + clippy.
**Commit:** `0f9040f`

### M-06: No Contribution Guidelines
**Before:** No `CONTRIBUTING.md` or `CODE_OF_CONDUCT.md`.
**After:** Both added with development setup, code standards, PR process, conventional commits.
**Commit:** `0f9040f`

### M-07: No Changelog
**Before:** No `CHANGELOG.md`. No git release tags.
**After:** `CHANGELOG.md` covering 0.1.0 → 0.1.2. Git tag `v0.1.2` created.
**Commit:** `0f9040f`

### M-08: Clippy & OSV Scanner Configuration
**Before:** No clippy configuration. No OpenSSF vulnerability scanning.
**After:** `clippy.toml` with MSRV config. OSV Scanner in CI (per-push) and weekly scheduled scan.
**Commit:** `daef74b`

---

## Version Impact

All 132 crates require republishing to crates.io:

| Category | Count | Old Version | New Version | Reason |
|----------|-------|-------------|-------------|--------|
| Workspace crates | ~70 | 0.1.0 | **0.1.1** | License + SAFETY comments |
| Explicit version crates | ~55 | 0.1.0 | **0.1.1** | License + security fixes |
| `vil_server_core` | 1 | 0.1.2 | **0.1.3** | AES-256-GCM, CORS, reorg |
| `vil_observer` | 1 | 0.1.3 | **0.1.4** | License fix |
| `vil_new_http` | 1 | 0.1.1 | **0.1.2** | License + upstream tracking |
| `vil_plugin_sdk` | 1 | 0.6.0 | 0.6.0 | No change needed |

**Republish command:**
```bash
while ./scripts/publish-batch.sh; do echo "--- Waiting 10 min ---"; sleep 600; done
```

---

## Dependencies Added

| Crate | Version | Purpose | Used In |
|-------|---------|---------|---------|
| `aes-gcm` | 0.10 | AES-256-GCM authenticated encryption | `vil_server_core` |
| `getrandom` | 0.2 | OS-level CSPRNG | `vil_server_core`, `vil_server_auth` |
| `sha2` | 0.10 | SHA-256 API key hashing | `vil_server_auth` |
| `subtle` | 2.5 | Constant-time comparison | `vil_server_auth` |

---

## Breaking Changes

1. **Secrets format incompatible.** Values encrypted with the old XOR cipher cannot be decrypted by the new AES-256-GCM implementation. Re-encrypt all secrets after upgrade.

2. **CORS disabled by default.** Applications that relied on the previous `cors: true` default must add `.cors_permissive()` to their `VilServer` builder.

3. **API key storage format changed.** Keys are now stored as SHA-256 hashes. Applications must re-register all API keys after upgrade (the plaintext key is still used in the `add_key()` call — only internal storage changed).

---

## Remaining Known Issues

| Issue | Severity | Status |
|-------|----------|--------|
| 8 advisories in transitive dependencies (alloy-dyn-abi, idna, lru) | MEDIUM | Tracked — waiting for upstream fixes |
| `vil_server_core` 14K lines — needs further decomposition | LOW | Modules reorganized; full crate extraction deferred |
| 2 examples broken (observer feature-gate) | LOW | Excluded from CI; fix tracked |
| ~1000 `unwrap()` calls in non-test code | LOW | Clippy lint enabled as warning |

---

## Verification

```bash
# Reproduce locally
cargo check --workspace --exclude vil-basic-ai-gw-demo --exclude vil-multi-pipeline-benchmark
cargo test --workspace --exclude vil-basic-ai-gw-demo --exclude vil-multi-pipeline-benchmark
cargo fmt --all -- --check
cargo clippy --workspace --exclude vil-basic-ai-gw-demo --exclude vil-multi-pipeline-benchmark
```

**CI:** https://github.com/OceanOS-id/VIL/actions

---

*Report generated 2026-03-29. For questions: security@vastar.id*
