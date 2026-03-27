# Phase 1 — Q3 2026: Storage & Database Expansion

> **⚠ MANDATORY: Read [COMPLIANCE.md](./COMPLIANCE.md) before implementing any crate in this phase.**
> Every crate must pass the full compliance checklist (P1–P10, testing, docs, pre-merge review).
> Non-compliant crates will be rejected regardless of functionality.

## Objective

Extend VIL's data layer beyond SQL+Redis to cover object storage, NoSQL, time-series, graph, and full-text search. Every new crate integrates with the existing `vil_db_semantic` IR and `vil_cache` abstraction.

---

## 1. Object Storage

### 1.1 `vil_storage_s3` — MinIO / S3-Compatible

**Priority**: High (most requested)

**Scope**:
- `S3Client` trait with `put_object`, `get_object`, `delete_object`, `list_objects`, `presigned_url`
- Multipart upload with configurable chunk size
- Streaming download (zero-copy to SHM when possible)
- Tri-Lane bridge: Trigger Lane receives upload events, Data Lane streams content, Control Lane handles errors/retries

**Dependencies**:
- `aws-sdk-s3` or `rusoto_s3` (evaluate: aws-sdk-s3 is official, prefer it)
- `vil_types`, `vil_rt`, `vil_obs`

**Implementation Plan**:
```
crates/vil_storage_s3/
├── Cargo.toml
├── README.md
├── src/
│   ├── lib.rs          — re-exports
│   ├── client.rs       — S3Client implementation
│   ├── config.rs       — S3Config (endpoint, region, credentials)
│   ├── multipart.rs    — multipart upload/download
│   ├── presigned.rs    — presigned URL generation
│   ├── stream.rs       — streaming get/put with SHM bridge
│   └── error.rs        — S3Error variants
├── tests/
│   ├── integration.rs  — requires MinIO container
│   └── mock.rs         — unit tests with mock S3
└── examples/
    └── s3_upload_download.rs
```

**Testing**: Docker-compose with MinIO container. CI runs integration tests.

**Estimated effort**: 3-5 days scaffolding (AI), 2 days quality review (human)

---

### 1.2 `vil_storage_gcs` — Google Cloud Storage

**Priority**: Medium

**Scope**:
- Same trait interface as `vil_storage_s3` (shared `StorageProvider` trait)
- GCS-specific: signed URLs, IAM-based auth, resumable uploads
- JSON API + gRPC API support

**Dependencies**:
- `google-cloud-storage` or raw `reqwest` + GCS JSON API
- `vil_types`, `vil_rt`, `vil_obs`

**Implementation Plan**:
```
crates/vil_storage_gcs/
├── src/
│   ├── lib.rs
│   ├── client.rs       — GcsClient
│   ├── config.rs       — service account JSON / workload identity
│   ├── resumable.rs    — resumable upload
│   └── error.rs
├── tests/
└── examples/
```

**Testing**: GCS emulator or mock. Real GCS tests gated behind `--features gcs-live`.

**Estimated effort**: 2-3 days

---

### 1.3 `vil_storage_azure` — Azure Blob Storage

**Priority**: Medium

**Scope**:
- `AzureBlobClient` implementing shared `StorageProvider` trait
- SAS token auth, managed identity
- Block blob + append blob support

**Dependencies**:
- `azure_storage_blobs` crate
- `vil_types`, `vil_rt`, `vil_obs`

**Estimated effort**: 2-3 days

---

### Shared Trait: `StorageProvider`

Define in `vil_types` or a new `vil_storage_core`:

```rust
#[async_trait]
pub trait StorageProvider: Send + Sync {
    async fn put(&self, bucket: &str, key: &str, body: Bytes) -> Result<PutResult>;
    async fn get(&self, bucket: &str, key: &str) -> Result<StreamBody>;
    async fn delete(&self, bucket: &str, key: &str) -> Result<()>;
    async fn list(&self, bucket: &str, prefix: &str) -> Result<Vec<ObjectMeta>>;
    async fn presigned_url(&self, bucket: &str, key: &str, ttl: Duration) -> Result<String>;
}
```

---

## 2. Database Expansion

### 2.1 `vil_db_mongo` — MongoDB

**Priority**: High

**Scope**:
- `MongoPool` with connection pooling
- CRUD operations with BSON serialization
- Aggregation pipeline builder
- Change Stream support (feeds into Trigger system in Phase 3)
- Integration with `vil_db_semantic` IR

**Dependencies**:
- `mongodb` (official Rust driver)
- `vil_types`, `vil_db_semantic`, `vil_obs`

**Implementation Plan**:
```
crates/vil_db_mongo/
├── src/
│   ├── lib.rs
│   ├── pool.rs         — MongoPool (connection management)
│   ├── crud.rs         — insert, find, update, delete
│   ├── aggregate.rs    — aggregation pipeline builder
│   ├── change.rs       — change stream listener
│   ├── bson_bridge.rs  — VIL types <-> BSON conversion
│   └── error.rs
├── tests/
│   └── integration.rs  — requires MongoDB container
└── examples/
    ├── mongo_crud.rs
    └── mongo_change_stream.rs
```

**Testing**: Docker MongoDB. Testcontainers integration.

**Estimated effort**: 4-5 days

---

### 2.2 `vil_db_clickhouse` — ClickHouse (OLAP)

**Priority**: Medium-High (analytics use cases)

**Scope**:
- Async query execution with streaming result sets
- Batch insert (columnar format)
- Materialized view management
- Integration with `vil_obs` for pipeline analytics storage

**Dependencies**:
- `clickhouse` crate (klickhouse or clickhouse-rs)
- `vil_types`, `vil_obs`

**Implementation Plan**:
```
crates/vil_db_clickhouse/
├── src/
│   ├── lib.rs
│   ├── client.rs       — ClickHouseClient
│   ├── query.rs        — parameterized queries
│   ├── insert.rs       — batch columnar insert
│   ├── migration.rs    — DDL management
│   └── error.rs
├── tests/
└── examples/
    └── analytics_pipeline.rs
```

**Estimated effort**: 3-4 days

---

### 2.3 `vil_db_dynamodb` — AWS DynamoDB

**Priority**: Medium

**Scope**:
- Table operations (CRUD, scan, query)
- GSI/LSI support
- DynamoDB Streams (for CDC trigger in Phase 3)
- Single-table design patterns

**Dependencies**:
- `aws-sdk-dynamodb`
- `vil_types`, `vil_rt`

**Estimated effort**: 3-4 days

---

### 2.4 `vil_db_cassandra` — Cassandra / ScyllaDB

**Priority**: Medium

**Scope**:
- Session management with load balancing policies
- Prepared statements with token-aware routing
- Lightweight transactions (LWT)
- Batch operations

**Dependencies**:
- `scylla` crate (works with both Cassandra and ScyllaDB)
- `vil_types`, `vil_rt`

**Estimated effort**: 3-4 days

---

### 2.5 `vil_db_timeseries` — InfluxDB / TimescaleDB

**Priority**: Medium (IoT + monitoring)

**Scope**:
- InfluxDB v2 API (write, query with Flux)
- TimescaleDB via existing `vil_db_sqlx` extension (hypertable, continuous aggregates)
- Common `TimeSeriesWriter` trait for both backends
- Integration with `vil_obs` for metrics storage

**Dependencies**:
- `influxdb2` or raw HTTP client
- `vil_db_sqlx` (for TimescaleDB path)
- `vil_types`, `vil_obs`

**Estimated effort**: 4-5 days (two backends)

---

### 2.6 `vil_db_neo4j` — Neo4j

**Priority**: Medium (complements `vil_graphrag`)

**Scope**:
- Bolt protocol driver
- Cypher query builder
- Transaction support
- Bridge to `vil_graphrag` for graph storage backend

**Dependencies**:
- `neo4rs` crate
- `vil_types`, `vil_graphrag`

**Estimated effort**: 3 days

---

### 2.7 `vil_db_elastic` — Elasticsearch / OpenSearch

**Priority**: Medium-High (full-text search, log analytics)

**Scope**:
- Index management (create, mapping, alias)
- CRUD + bulk operations
- Search DSL builder (bool query, aggregations, highlighting)
- Scroll/search_after for large result sets
- Bridge to `vil_rag` as retrieval backend

**Dependencies**:
- `elasticsearch` crate or raw HTTP
- `vil_types`, `vil_rag`

**Estimated effort**: 4-5 days

---

## Milestone Checklist

- [ ] Define shared `StorageProvider` trait
- [ ] `vil_storage_s3` — implemented + tested with MinIO
- [ ] `vil_storage_gcs` — implemented + tested with emulator
- [ ] `vil_storage_azure` — implemented + tested with Azurite
- [ ] `vil_db_mongo` — implemented + tested with Docker
- [ ] `vil_db_clickhouse` — implemented + tested with Docker
- [ ] `vil_db_dynamodb` — implemented + tested with DynamoDB Local
- [ ] `vil_db_cassandra` — implemented + tested with Docker
- [ ] `vil_db_timeseries` — implemented + tested (both backends)
- [ ] `vil_db_neo4j` — implemented + tested with Docker
- [ ] `vil_db_elastic` — implemented + tested with Docker
- [ ] All crates have README, examples, benchmarks
- [ ] All crates registered in workspace Cargo.toml
- [ ] Integration with `vil_db_semantic` IR verified
- [ ] `vil init` templates updated to offer new DB options
