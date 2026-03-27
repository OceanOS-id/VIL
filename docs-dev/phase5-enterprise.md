# Phase 5 — H2 2027: Enterprise

> **⚠ MANDATORY: Read [COMPLIANCE.md](./COMPLIANCE.md) before implementing any crate in this phase.**
> Every crate must pass the full compliance checklist (P1–P10, testing, docs, pre-merge review).
> Non-compliant crates will be rejected regardless of functionality.

## Objective

Harden VIL for enterprise production deployment: multi-tenancy, compliance, observability export, edge computing, and community ecosystem growth.

---

## 1. Multi-Tenancy & Namespace Isolation

### 1.1 `vil_tenant` — Multi-Tenant Runtime

**Priority**: High (cloud + SaaS prerequisite)

**Scope**:
- Tenant namespace: isolated ExchangeHeap regions per tenant
- Tenant-scoped process registry (processes from tenant A cannot see tenant B)
- Resource quotas per tenant (SHM size, CPU, connections)
- Tenant context propagation through Tri-Lane (tenant_id in every descriptor)
- Tenant-aware routing in `vil_server_mesh`

**Tri-Lane compliance**:
- Trigger Lane carries `tenant_id` in every `TriggerFired` event
- Data Lane: ExchangeHeap regions are partitioned per tenant
- Control Lane: tenant isolation enforced (no cross-tenant control signals)

**Implementation Plan**:
```
crates/vil_tenant/
├── src/
│   ├── lib.rs
│   ├── namespace.rs    — TenantNamespace (isolated SHM region + registry)
│   ├── quota.rs        — ResourceQuota (SHM, CPU, connections)
│   ├── context.rs      — TenantContext propagation through ServiceCtx
│   ├── router.rs       — tenant-aware mesh routing
│   ├── types.rs        — #[vil_state] TenantInfo, #[vil_fault] TenantFault
│   └── error.rs
├── tests/
│   ├── isolation.rs    — verify cross-tenant isolation
│   └── quota.rs        — verify resource limits
└── examples/
    └── multi_tenant_server.rs
```

**Semantic types**:
```rust
#[vil_state(layout = "flat")]
pub struct TenantContext {
    pub tenant_id: u64,
    pub namespace_hash: u64,
    pub quota_shm_bytes: u64,
    pub quota_max_processes: u32,
}

#[vil_fault]
pub enum TenantFault {
    QuotaExceeded { tenant_id: u64, resource: u8, current: u64, limit: u64 },
    NamespaceNotFound { tenant_id: u64 },
    IsolationViolation { source_tenant: u64, target_tenant: u64 },
}
```

**Estimated effort**: 7-10 days

---

## 2. Compliance Connectors

### 2.1 `vil_audit` — Audit Trail

**Priority**: High (regulated industries — banking, healthcare, government)

**Scope**:
- Immutable audit log of all ownership transfers (who accessed what, when)
- Tri-Lane Control Lane events automatically captured
- Tamper-evident log (hash chain — each entry references previous hash)
- Export formats: JSON Lines, CEF (Common Event Format), OCSF
- Storage backends: local file, S3 (via `vil_storage_s3`), database
- Retention policy (configurable TTL, archival)

**Zero-copy compliance**:
- Audit entries are `#[vil_event(layout = "flat")]` — written directly to SHM
- Audit writer is a `ServiceProcess` consuming from Control Lane tap
- No allocation on hot path — audit is a zero-copy observer

**Implementation Plan**:
```
crates/vil_audit/
├── src/
│   ├── lib.rs
│   ├── logger.rs       — AuditLogger (ServiceProcess, Control Lane tap)
│   ├── chain.rs        — hash chain integrity
│   ├── format.rs       — JSON Lines, CEF, OCSF formatters
│   ├── storage.rs      — pluggable backends (file, S3, DB)
│   ├── retention.rs    — TTL, archival policies
│   ├── types.rs        — #[vil_event] AuditEntry
│   └── error.rs
├── tests/
│   ├── integrity.rs    — tamper detection test
│   └── retention.rs    — TTL enforcement test
└── examples/
    └── audit_trail_pipeline.rs
```

**Semantic types**:
```rust
#[vil_event(layout = "flat")]
pub struct AuditEntry {
    pub sequence: u64,
    pub prev_hash: [u8; 32],
    pub timestamp_ns: u64,
    pub tenant_id: u64,
    pub actor_hash: u64,        // hash of actor identity
    pub action: u8,             // 0=Read, 1=Write, 2=Delete, 3=Transfer, 4=Escalate
    pub resource_hash: u64,     // hash of resource identifier
    pub process_id: u64,
    pub outcome: u8,            // 0=Success, 1=Denied, 2=Error
}
```

**Estimated effort**: 5-7 days

---

### 2.2 `vil_gdpr` — GDPR / Privacy Tooling

**Priority**: Medium-High (EU compliance)

**Scope**:
- Data Subject Access Request (DSAR) handler
- Right to erasure (scan + delete across configured stores)
- Consent management (record + enforce consent status)
- Data classification tags on `#[vil_state]` types
- PII detection integration (reuses `vil_guardrails` PII detector)
- Data Processing Agreement (DPA) audit reports

**Implementation Plan**:
```
crates/vil_gdpr/
├── src/
│   ├── lib.rs
│   ├── dsar.rs         — Data Subject Access Request handler
│   ├── erasure.rs      — right to erasure across stores
│   ├── consent.rs      — consent recording + enforcement
│   ├── classify.rs     — data classification tags
│   ├── report.rs       — DPA audit report generation
│   ├── types.rs        — #[vil_state] ConsentRecord, #[vil_fault] GdprFault
│   └── error.rs
├── tests/
└── examples/
    └── gdpr_compliant_pipeline.rs
```

**Estimated effort**: 7-10 days

---

## 3. Observability Export

### 3.1 `vil_otel` — OpenTelemetry Native Export

**Priority**: High (industry standard observability)

**Scope**:
- OTLP exporter (gRPC + HTTP) for traces, metrics, logs
- VIL `vil_obs` metrics → OpenTelemetry metrics bridge
- VIL `#[trace_hop]` spans → OpenTelemetry trace spans
- VIL `#[vil_fault]` → OpenTelemetry log events
- Resource attributes (service name, version, tenant_id)
- Batch export with configurable flush interval

**Zero-copy compliance**:
- OTel export is a `ServiceProcess` — reads from obs buffer, no hot-path allocation
- Metric aggregation happens in SHM counters (already zero-copy via `vil_obs`)
- Export serialization (protobuf) happens at boundary — acceptable Copy

**Dependencies**:
- `opentelemetry` + `opentelemetry-otlp` crates
- `vil_obs`, `vil_types`, `vil_rt`

**Implementation Plan**:
```
crates/vil_otel/
├── src/
│   ├── lib.rs
│   ├── metrics.rs      — vil_obs → OTel metrics bridge
│   ├── traces.rs       — trace_hop → OTel spans bridge
│   ├── logs.rs         — vil_fault → OTel log events
│   ├── exporter.rs     — OTLP gRPC + HTTP exporter
│   ├── resource.rs     — OTel resource attributes
│   └── error.rs
├── tests/
│   └── integration.rs  — Docker Jaeger/Grafana Tempo
└── examples/
    └── otel_export_pipeline.rs
```

**Estimated effort**: 4-5 days

---

### 3.2 Grafana Dashboard Templates

**Priority**: Medium (UX for ops teams)

**Scope**:
- Pre-built Grafana dashboards as JSON provisioning files
- Dashboards for: pipeline throughput, SHM utilization, Tri-Lane latency, DB pool, MQ lag
- Prometheus data source (via OTel → Prometheus remote write)
- Alert rules for: SHM exhaustion, pipeline stall, error rate spike

**Deliverables**:
```
dashboards/grafana/
├── vil-pipeline-overview.json
├── vil-shm-utilization.json
├── vil-tri-lane-latency.json
├── vil-database-pool.json
├── vil-mq-consumer-lag.json
├── vil-ai-gateway.json
└── alerts/
    ├── shm-exhaustion.yaml
    ├── pipeline-stall.yaml
    └── error-rate.yaml
```

**Estimated effort**: 3-4 days

---

## 4. Edge Deployment

### 4.1 `vil_edge_deploy` — ARM / RISC-V Optimized Builds

**Priority**: Medium (IoT, smart city, industrial)

**Scope**:
- Cross-compilation targets: `aarch64-unknown-linux-gnu`, `armv7-unknown-linux-gnueabihf`, `riscv64gc-unknown-linux-gnu`
- Minimal runtime profile (reduced SHM, single-core scheduler)
- `vil build --target edge-arm64` CLI command
- Binary size optimization (LTO + strip + panic=abort)
- Edge-specific config profile (low memory, limited connectivity)
- OTA update support (checksum verify, atomic swap)

**Implementation Plan**:
```
crates/vil_edge_deploy/
├── src/
│   ├── lib.rs
│   ├── profile.rs      — edge runtime profile (minimal SHM, single-core)
│   ├── cross.rs        — cross-compilation helpers
│   ├── optimize.rs     — binary size optimization
│   ├── ota.rs          — over-the-air update (download + verify + swap)
│   └── error.rs
├── tests/
└── examples/
    └── edge_iot_gateway.rs
```

**Edge config profile**:
```yaml
profile: edge
shm:
  size: 4MB              # vs 256MB production
  pages: 64              # vs 4096
scheduler:
  mode: single_core      # no work-stealing
  max_processes: 16      # vs unlimited
network:
  reconnect_interval: 30s
  offline_buffer: 1MB    # buffer events when disconnected
```

**Estimated effort**: 5-7 days

---

## 5. Plugin Marketplace with Community Review

### 5.1 Marketplace Backend Enhancement

**Priority**: Medium (depends on Phase 4 B3 marketplace)

**Scope**:
- Automated compliance checking on publish:
  - Lint Cargo.toml metadata
  - Check `#[vil_fault]` usage (not bare `thiserror`)
  - Check `TriggerSource` / `MqBridge` trait implementation
  - Check `ServiceProcess` registration
  - Run `cargo clippy` + `cargo test`
- Human review queue for first-time publishers
- Verified publisher badge (after 3+ approved submissions)
- Security scanning (no `unsafe` without justification, dependency audit)
- Compatibility matrix (which VIL version each plugin supports)

**Implementation Plan**:
```
marketplace/
├── backend/
│   ├── src/
│   │   ├── review/
│   │   │   ├── auto_lint.rs        — automated compliance checks
│   │   │   ├── security_scan.rs    — unsafe audit, dependency check
│   │   │   └── queue.rs            — human review queue
│   │   ├── publish/
│   │   │   ├── upload.rs           — crate upload + extract
│   │   │   ├── verify.rs           — checksum + signature
│   │   │   └── index.rs            — search index update
│   │   └── api/
│   │       ├── search.rs
│   │       ├── download.rs
│   │       └── review.rs
│   └── Cargo.toml
└── frontend/                        — React (shared with vastar.id website)
```

**Estimated effort**: 2-3 weeks (builds on Phase 4 B3)

---

## Development Order

1. **`vil_otel`** — most requested, immediate production value (week 1-2)
2. **Grafana dashboards** — companion to OTel (week 2)
3. **`vil_tenant`** — prerequisite for Cloud (week 3-4)
4. **`vil_audit`** — prerequisite for compliance (week 4-5)
5. **`vil_edge_deploy`** — independent track (week 5-6)
6. **`vil_gdpr`** — builds on audit + tenant (week 6-8)
7. **Marketplace review system** — builds on Phase 4 marketplace (week 8-10)

---

## Milestone Checklist

- [ ] `vil_otel` — OTLP export for traces + metrics + logs
- [ ] Grafana dashboards — 6 dashboards + 3 alert rules
- [ ] `vil_tenant` — namespace isolation + quota + context propagation
- [ ] `vil_audit` — hash-chain audit trail + 3 export formats
- [ ] `vil_gdpr` — DSAR + erasure + consent management
- [ ] `vil_edge_deploy` — ARM64 + ARMv7 + RISC-V builds
- [ ] Marketplace review — automated compliance lint + human review queue
- [ ] All crates use `#[vil_fault]` for errors
- [ ] All crates use `ServiceProcess` for runtime behavior
- [ ] All crates have Tri-Lane integration documented
- [ ] COMPLIANCE.md checklist passed for every crate
- [ ] Website docs updated for enterprise features
