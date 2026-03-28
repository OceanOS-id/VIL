# vil-server API Reference

**Version:** 5.3.0 | **Tests:** 1,425+ | **Last Updated:** 2026-03-27
**Location:** `docs/vil-server/API-REFERENCE-SERVER.md`

Complete module-by-module API reference for all VIL crates.

---

## vil_server (Umbrella)

Re-exports all sub-crates. Single dependency for users.

```rust
use vil_server::prelude::*;
```

**Prelude exports:** `VilApp`, `ServiceProcess`, `ServiceCtx`, `VxMeshConfig`, `VxLane`, `Method`, `VilModel`, `DeriveVilError`, `vil_handler`, `VilSseEvent`, `VilWsEvent`, `vil_endpoint`, `vil_app`, `vil_service`, `vil_service_state`, `VilServer`, `Json`, `Path`, `Query`, `State`, `StatusCode`, `IntoResponse`, `get`, `post`, `put`, `delete`, `patch`, `Router`, `ServiceDef`, `Visibility`, `ShmSlice`, `ShmResponse`, `ShmJson`, `ShmContext`, `blocking_with`, `Valid`, `HandlerResult`, `RequestId`, `VilResponse`, `NoContent`, `Lane`, `MeshMode`, `MeshBuilder`, `JwtAuth`, `RateLimit`, `Serialize`, `Deserialize`

---

## vil_server_core

### VilApp (Process-Oriented — Recommended)

| Type | API | Description |
|------|-----|-------------|
| `VilApp` | `.new(name)` `.port(u16)` `.metrics_port(u16)` `.profile("prod")` `.heap_size(bytes)` `.service(ServiceProcess)` `.mesh(VxMeshConfig)` `.plugin(impl VilPlugin)` `.sidecar(SidecarConfig)` `.observer(bool)` `.no_cors()` `.max_body_size(bytes)` `.contract_json()` `.run().await` | Process topology builder |
| `ServiceProcess` | `.new(name)` `.visibility(Visibility)` `.prefix(path)` `.endpoint(Method, path, handler)` `.state(T)` `.emits::<T>()` `.faults::<T>()` `.manages::<T>()` | Service-as-Process |
| `ServiceCtx` | `.state::<T>()` `.send(to, data)` `.trigger(to, data)` `.control(to, signal)` `.tri_lane()` | Process-aware context (axum extractor) |
| `VxMeshConfig` | `.new()` `.route(from, to, VxLane)` `.backpressure(svc, limit)` | Tri-Lane mesh routing |
| `VxLane` | `Trigger` `Data` `Control` | Lane selection |

### VilServer (Legacy — Backward Compatible)

| Type | API | Description |
|------|-----|-------------|
| `VilServer` | `.new(name)` `.port(u16)` `.metrics_port(u16)` `.route(path, handler)` `.service_def(def)` `.nest(path, router)` `.merge(router)` `.observer(bool)` `.no_cors()` `.run().await` | Server builder |
| `AppState` | `.new(name)` `.new_shared(name)` `.runtime()` `.shm()` `.metrics()` `.process_registry()` `.handler_metrics()` `.capsule_registry()` `.span_collector()` `.custom_metrics()` `.error_tracker()` `.profiler()` `.config_reloader()` `.uptime_secs()` `.name()` `.version()` `.sync_metrics()` | Shared state |
| `ServiceDef` | `.new(name, router)` `.prefix(path)` `.visibility(vis)` `.internal()` | Named service |
| `Visibility` | `Public` `Internal` | Service visibility |

### Extractors

| Type | From | Description |
|------|------|-------------|
| `RequestId` | Header `X-Request-Id` (auto-generated if absent) | Unique request ID |
| `ShmContext` | State | `.available` `.region_count()` `.region_stats()` `.heap()` |
| `ShmSlice` | Request body | `.as_bytes()` `.len()` `.json::<T>()` `.text()` `.region_id()` `.offset()` `.into_bytes()` |
| `AcceptHeader` | `Accept` header | `.preferred()` `.wants_json()` `.accepts(ct)` |
| `ApiVersion` | URL/Header/Accept | `.major` `.minor` `.source` `.is_v1()` `.is_v2()` |

### Responses

| Type | API | Description |
|------|-----|-------------|
| `VilResponse<T>` | `::ok(data)` `::created(data)` | JSON with status |
| `NoContent` | — | 204 response |
| `ShmResponse` | `::ok(bytes)` `::json(data)` `.write_to_shm(heap)` | SHM-backed response |
| `ShmJson<T>` | `::new(data)` `.with_shm(heap)` | JSON + SHM write-through |
| `VilError` | `::bad_request(msg)` `::not_found(msg)` `::unauthorized(msg)` `::internal(msg)` `::validation(fields)` `::rate_limited()` | RFC 7807 errors |

### Middleware

| Module | API | Description |
|--------|-----|-------------|
| `middleware` | `request_tracker` | X-Request-Id, timing, error tracking |
| `obs_middleware` | `handler_metrics` / `HandlerMetricsRegistry` | Per-route Prometheus metrics |
| `trace_middleware` | `tracing_middleware` | W3C distributed tracing |
| `timeout` | `TimeoutLayer::new(dur)` `::from_secs(n)` | Request timeout (Tower Layer) |
| `compression` | `compression_layer()` / `CompressionConfig` | Gzip/deflate |
| `request_log` | `request_logger` | Structured logging (severity-based) |
| `tls` | `hsts_middleware` / `TlsConfig` | HSTS enforcement |
| `middleware_stack` | `MiddlewareStack::new()` `.timeout()` `.compression()` `.security_headers()` `.apply()` | Composition builder |

### Observability

| Module | API | Description |
|--------|-----|-------------|
| `otel` | `TraceId` `SpanId` `SpanBuilder` `SpanCollector` `TraceContext` | OpenTelemetry |
| `custom_metrics` | `CustomMetrics` `.register_counter()` `.inc()` `.gauge_set()` `.observe()` `.to_prometheus()` | User metrics |
| `error_tracker` | `ErrorTracker` `.record()` `.recent()` `.top_patterns()` | Error aggregation |
| `alerting` | `AlertEngine` `.add_rule()` `.evaluate()` | Threshold alerts |
| `profiler` | `ServerProfiler` `.snapshot()` | RSS, connections, throughput |
| `diagnostics` | `diagnostics_router()` | /admin/diagnostics, /traces, /errors, /shm |

### WASM

| Module | API | Description |
|--------|-----|-------------|
| `capsule_handler` | `CapsuleRegistry` `.load_from_file()` `.reload()` `.get_module()` | WASM module registry |
| `wasm_host` | `WasmHostRegistry` `WasmCapability` `WasmHandlerContext` `WasmHandlerResponse` | Host function capabilities |
| `wasm_dispatch` | `dispatch_to_wasm()` `WasmPool` | Request → capsule → response |
| `wasm_shm_bridge` | `WasmShmBridge` `.stage_input()` `.read_output()` `.reset()` | WASM ↔ SHM memory bridge |

### Advanced

| Module | API | Description |
|--------|-----|-------------|
| `cache` | `Cache<K,V>` `.get()` `.put()` `.put_with_ttl()` `.cleanup_expired()` `.stats()` | LRU + TTL cache |
| `feature_flags` | `FeatureFlags` `.define()` `.is_enabled()` `.is_enabled_for()` `.toggle()` | Runtime flags |
| `scheduler` | `Scheduler` `.every()` `.once()` `.cancel()` `.list_jobs()` | Background jobs |
| `http_client` | `HttpClientPool` `.record_request()` `.host_stats()` | Connection pool |
| `idempotency` | `IdempotencyStore` `.get()` `.put()` `.evict_expired()` | Request dedup |
| `coalescing` | `Coalescer<K,V>` `.submit()` | Request batching |
| `streaming` | `SseHub` `.broadcast()` `.subscribe()` / `WsHub` `.broadcast()` `.subscribe()` / `StreamingBody` | SSE + WebSocket fan-out |
| `plugin` | `PluginRegistry` `.register()` `.list()` | Native plugins |
| `rolling_restart` | `RestartCoordinator` `.start_drain()` `.wait_for_drain()` | Zero-downtime |
| `hot_reload` | `ConfigReloader` / `reload_router()` | Runtime config reload |
| `playground` | `playground_router()` | Embedded API explorer |
| `api_versioning` | `ApiVersion` extractor | URL/header/accept versioning |
| `multi_protocol` | `detect_protocol()` `MultiProtocolConfig` | HTTP/gRPC/WS detection |
| `middleware_dsl` | `MiddlewarePipeline` `.from_yaml()` `.validate()` | YAML middleware config |
| `etag` | `generate_etag()` `matches_etag()` | Conditional requests |
| `retry` | `RetryPolicy` `RetryStrategy` `retry_async()` | Outbound retry |

---

## vil_server_auth

| Module | API | Description |
|--------|-----|-------------|
| `jwt` | `JwtAuth` `.new(secret)` `.optional()` `.validate_token()` / `jwt_middleware` | JWT Bearer auth |
| `rate_limit` | `RateLimit` `.new(max, window)` `.check(ip)` `.remaining(ip)` | Token bucket per-IP |
| `circuit_breaker` | `CircuitBreaker` `.check()` `.record_success()` `.record_failure()` `.state()` `.status()` `.reset()` | 3-state FSM |
| `oauth2` | `OAuth2Client` `.authorization_url()` `.cache_token()` `.get_cached_token()` / `OAuth2Config` `OidcClaims` | OAuth2/OIDC |
| `security` | `security_headers` middleware / `BodySizeLimit` / `BruteForceProtection` / `SecurityStatus` | OWASP headers |
| `api_key` | `ApiKeyAuth` `.add_key()` `.add_key_scoped()` `.revoke_key()` `.validate()` / `api_key_middleware` | API key auth |
| `ip_filter` | `IpFilter` `::allowlist()` `::blocklist()` `.add_ip()` `.add_cidr()` `.is_allowed()` | IP filtering |
| `rbac` | `RbacPolicy` `.add_role()` `.check_permission()` `.effective_permissions()` / `Role` `.permission()` | RBAC |
| `csrf` | `CsrfProtection` `.generate_token()` `.needs_check()` `.validate()` / `CsrfConfig` | CSRF protection |
| `audit` | `AuditLog` `.record()` `.recent()` `.count()` / `AuditEvent` `AuditEventType` | Security audit log |
| `session` | `SessionManager` `.create()` `.get()` `.update()` `.destroy()` `.cleanup_expired()` `.cookie_header()` | Cookie sessions |

---

## vil_server_mesh

| Module | API | Description |
|--------|-----|-------------|
| `Lane` | `Trigger` `Data` `Control` | Tri-Lane types |
| `MeshBuilder` | `.route(from, to, lane, mode)` `.build()` | Programmatic mesh config |
| `channel` | `MeshSender` `.send()` / `MeshReceiver` `.recv()` | Tokio mpsc channels |
| `shm_bridge` | `ShmMeshChannel` `.send()` / `ShmMeshReceiver` `.recv()` | SHM zero-copy channels |
| `tri_lane` | `TriLaneRouter` `.register_route()` `.send()` `.apply_config()` / `TcpTransport` / `Transport` | Per-pair Trigger/Data/Control |
| `router` | `MeshRouter` `.register_service()` `.sender_for()` | Service channel management |
| `discovery` | `ServiceDiscovery` trait / `ConfigDiscovery` / `Endpoint` `HealthStatus` | Pluggable discovery |
| `shm_discovery` | `ShmDiscovery` `.register_local()` `.is_co_located()` | Auto co-located discovery |
| `yaml_config` | `VilServerYaml` `.from_file()` `.from_str()` `.validate()` `.to_mesh_config()` | YAML service definition |
| `backpressure` | `BackpressureController` `.request_enter()` `.request_exit()` `.is_accepting()` / `UpstreamThrottle` / `BackpressureSignal` | Adaptive backpressure |
| `mq_adapter` | `MqAdapter` trait / `NatsAdapter` / `MqMessage` `MqSubscription` | Message queue integration |
| `pipeline_dag` | `PipelineDag` `.add_node()` `.validate()` `.plan()` `.entry_nodes()` `.exit_nodes()` | DAG execution |
| `scatter_gather` | `ScatterGather` `.target()` `.strategy()` `.execute()` / `GatherStrategy` | Fan-out/fan-in |
| `dlq` | `DeadLetterQueue` `.enqueue()` `.recent()` `.get()` `.mark_replayed()` | Failed message handling |
| `typed_rpc` | `RpcRegistry` `.register()` `.invoke()` / `RpcClient` `.call()` | Typed inter-service RPC |
| `event_bus` | `EventBus` `.publish()` `.subscribe()` `.topic_count()` | In-process pub/sub |
| `cqrs` | `CqrsDispatcher` `.register_command()` `.register_query()` `.dispatch_command()` `.dispatch_query()` | Command/query separation |
| `load_balancer` | `LoadBalancer` `.next()` / `LbStrategy` (RoundRobin, LeastConn, Weighted, Canary) | Client-side LB |

---

## vil_server_web

| Module | API | Description |
|--------|-----|-------------|
| `Valid<T>` | Extractor — auto-validates JSON body via `validator` crate | Request validation |
| `HandlerError` | Type alias for `VilError` | Error type |
| `HandlerResult<T>` | `Result<T, HandlerError>` | Handler return type |
| `openapi` | `OpenApiBuilder` `.get()` `.post()` `.put()` `.delete()` `.build_json()` | OpenAPI 3.0 generation |

---

## vil_server_config

Precedence: Code Default → YAML → Profile → ENV (`VIL_*`)

### Loading

| Type | API | Description |
|------|-----|-------------|
| `ServerConfig` | `.from_file(path)` `.from_file_with_env(path)` `.from_env()` `.apply_profile()` | Lightweight config (VX_APP) |
| `FullServerConfig` | `.from_file(path)` `.from_file_with_env(path)` `.from_str(yaml)` `.apply_profile()` `.apply_env_overrides()` `.parse_size("64MB")` | Full production config |
| `Profile` | `Dev` `Staging` `Prod` `Custom(String)` / `.from_str(s)` `.apply(config)` `.is_dev()` `.is_prod()` `.default_log_level()` | Environment profiles |

### Configuration Sections (FullServerConfig)

| Section | Struct | Key Fields |
|---------|--------|------------|
| `server` | `ServerSection` | `port`, `host`, `workers`, `metrics_port`, `max_body_size`, `request_timeout_secs`, `graceful_shutdown_timeout_secs` |
| `logging` | `LogSection` | `level`, `format`, `modules` (per-module overrides) |
| `shm` | `ShmSection` | `enabled`, `pool_size`, `reset_threshold_pct`, `check_interval`, `query_cache` |
| `mesh` | `MeshSection` | `mode`, `channels` (trigger/data/control buffer_size + shm_region_size), `discovery`, `routes` |
| `pipeline` | `PipelineSection` | `queue_capacity`, `session_timeout_secs`, `max_concurrent` |
| `database` | `DatabaseSection` | `postgres` (url, max/min_connections, timeouts), `redis` (url, pool_size) |
| `mq` | `MqSection` | `nats` (url, max_reconnects), `kafka` (brokers, group_id), `mqtt` (host, port, client_id) |
| `middleware` | `MiddlewareSection` | request_tracker, handler_metrics, tracing, cors, compression, timeout, security_headers, hsts |
| `security` | `SecuritySection` | jwt, rate_limit, csrf, brute_force |
| `session` | `SessionSection` | cookie_name, ttl_secs, http_only, secure, same_site |
| `observability` | `ObservabilitySection` | error_tracker, span_collector, profiler |
| `performance` | `PerformanceSection` | metrics_sample_rate, trace_sample_rate, idempotency |
| `grpc` | `GrpcServerSection` | enabled, port, max_message_size, health_check, reflection |
| `graphql` | `GraphqlSection` | enabled, playground, max_depth, max_complexity, introspection |
| `admin` | `AdminSection` | playground, diagnostics, hot_reload, plugin_gui |

### Environment Variables

| Variable | Maps To | Example |
|----------|---------|---------|
| `VIL_PROFILE` | `profile` | `prod` |
| `VIL_SERVER_PORT` | `server.port` | `8080` |
| `VIL_SERVER_HOST` | `server.host` | `0.0.0.0` |
| `VIL_WORKERS` | `server.workers` | `8` |
| `VIL_LOG_LEVEL` | `logging.level` | `warn` |
| `VIL_SHM_POOL_SIZE` | `shm.pool_size` | `256MB` |
| `VIL_SHM_RESET_PCT` | `shm.reset_threshold_pct` | `90` |
| `VIL_SHM_CHECK_INTERVAL` | `shm.check_interval` | `1024` |
| `VIL_DATABASE_URL` | `database.postgres.url` | `postgres://...` |
| `VIL_DATABASE_MAX_CONNECTIONS` | `database.postgres.max_connections` | `50` |
| `VIL_REDIS_URL` | `database.redis.url` | `redis://...` |
| `VIL_NATS_URL` | `mq.nats.url` | `nats://...` |
| `VIL_KAFKA_BROKERS` | `mq.kafka.brokers` | `kafka:9092` |
| `VIL_MQTT_HOST` | `mq.mqtt.host` | `mqtt` |
| `VIL_PIPELINE_QUEUE_CAPACITY` | `pipeline.queue_capacity` | `4096` |

### Profile Presets

| Profile | SHM | Logging | DB Pool | Admin | Security |
|---------|-----|---------|---------|-------|----------|
| `dev` | 8MB, check/64 | debug, text | 5 conn | all on | off |
| `staging` | 64MB, check/256 | info, json | 20 conn | selective | rate limit on |
| `prod` | 256MB, check/1024 | warn, json | 50 conn | all off | hardened |

Reference: `vil-server.reference.yaml`

---

## vil_server_db

| Type | API | Description |
|------|-----|-------------|
| `DbPool` trait | `.acquire()` `.health_check()` `.close()` | Database pool interface |
| `Transaction<C>` | `.conn()` `.conn_mut()` `.commit()` | Auto-rollback transaction |
| `DbConfig` | `url`, `max_connections`, `connect_timeout_secs` | DB configuration |

---

## vil_server_test

| Type | API | Description |
|------|-----|-------------|
| `TestClient` | `.new(router)` `.get(path)` `.post_json(path, body)` `.delete(path)` `.send(req)` | Integration test client |
| `TestResponse` | `.status` `.text()` `.json::<T>()` `.assert_ok()` `.assert_created()` `.assert_not_found()` | Response assertions |
| `BenchRunner` | `.new(router)` `.requests(n)` `.concurrency(n)` `.path(p)` `.post(body)` `.run()` | Benchmark runner |

---

## vil_db_semantic (Compile-Time IR, Zero-Cost)

| Module | API | Runtime Cost |
|--------|-----|-------------|
| `DatasourceRef` | `::new(name)` `.name()` | 0 — `&'static str` |
| `TxScope` | `ReadOnly` `ReadWrite` `RequiresNew` `JoinIfPresent` `None` | 0 — 1 byte enum |
| `DbCapability` | `BASIC_CRUD` `TRANSACTIONS` `RELATIONS` `BULK_INSERT` `STREAMING_CURSOR` `JSON_QUERY` `FULL_TEXT_SEARCH` `NESTED_TX` `REPLICA_READ` `MIGRATION` `.contains()` `.union()` `SQL_STANDARD` `ORM_FULL` | 0 — u32 bitflag |
| `PortabilityTier` | `P0` (portable) `P1` (capability-gated) `P2` (provider-specific) | 0 — 1 byte |
| `CachePolicy` | `None` `Ttl(u32)` `InvalidateOnWrite` `SharedAcrossServices` | 0 — 8 bytes |
| `VilEntityMeta` | `const TABLE` `SOURCE` `PRIMARY_KEY` `FIELDS` `PORTABILITY` `CACHE_POLICY` | 0 — all const |
| `CrudRepository<T>` | `.find_by_id()` `.find_all()` `.insert()` `.update()` `.delete()` `.count()` `.exists()` | 1 vtable call |
| `DbProvider` | `.name()` `.capabilities()` `.health_check()` `.find_one()` `.find_many()` `.insert()` `.update()` `.delete()` `.count()` `.execute_raw()` | 1 vtable call |
| `ToSqlValue` | `Null` `Bool(bool)` `Int(i64)` `Float(f64)` `Text(String)` `Bytes(Vec<u8>)` | Stack enum |
| `DatasourceRegistry` | `.register()` `.resolve()` `.list()` `.health_check_all()` `.count()` | ~10ns DashMap get |
| `DbError` | `NotFound` `ConnectionFailed` `QueryFailed` `CapabilityMissing` `SchemaValidationFailed` `ProviderError` `Timeout` | Stack enum |

---

## vil_db_macros

| Macro | Input | Output | Cost |
|-------|-------|--------|------|
| `#[derive(VilEntity)]` | Struct with `#[vil(source, table, primary_key)]` | `impl VilEntityMeta` with `const` values | **Zero runtime** |

---

## vil_cache

| Module | API | Backend |
|--------|-----|---------|
| `VilCache` trait | `.get(key)` `.set(key, value, ttl)` `.del(key)` `.exists(key)` `.get_json()` `.set_json()` | Trait (1 vtable call) |
| `ShmCacheBackend` | `::new(shm_query_cache)` | ExchangeHeap zero-copy |
| `RedisCacheBackend` | `::new(redis_cache)` | vil_db_redis adapter |

---

## vil_db_sqlx

| Type | API | Description |
|------|-----|-------------|
| `SqlxPool` | `::connect(name, config)` `.inner()` `.execute_raw()` `.size_info()` `.close()` | impl DbPool via sqlx Any |
| `MultiPoolManager` | `.add_pool()` `.get()` `.get_for_service()` `.remove_pool()` `.health_check_all()` `.prometheus_metrics()` | Per-service pool manager |
| `SqlxConfig` | `::postgres(url)` `::mysql(url)` `::sqlite(url)` `.max_connections()` | Pool configuration |
| `DbConn` | `.pool()` `.execute()` `.metrics()` `.size_info()` | Handler DI |
| `PoolMetrics` | `.record_query()` `.record_acquire()` `.snapshot()` `.to_prometheus()` | Atomic counters |

---

## vil_db_sea_orm

| Type | API | Description |
|------|-----|-------------|
| `SeaOrmPool` | `::connect(name, config)` `.conn()` `.execute_raw()` `.close()` | impl DbPool via sea-orm |
| `SeaOrmConfig` | `::postgres(url)` `::mysql(url)` `::sqlite(url)` `.max_connections()` | ORM configuration |
| `MigrationRunner` trait | `.pending()` `.applied()` `.run_pending()` `.rollback_last()` `.status()` | Migration management |

---

## vil_db_redis

| Type | API | Description |
|------|-----|-------------|
| `RedisPool` | `::connect(name, config)` `.get()` `.set()` `.del()` `.keys_count()` | impl DbPool (in-memory stub) |
| `RedisCache` | `.get()` `.set(key, value, ttl)` `.del()` `.set_json()` `.get_json()` `.cleanup_expired()` | TTL cache with JSON helpers |
| `RedisConfig` | `::new(url)` | Connection configuration |

---

## vil_grpc

| Type | API | Description |
|------|-----|-------------|
| `GrpcGatewayBuilder` | `::new()` `.listen(port)` `.health_check(bool)` `.reflection(bool)` `.max_message_size(bytes)` `.build()` `.addr()` | 5-line gRPC server builder |
| `GrpcServerConfig` | `port` `max_message_size` `health_check` `reflection` `max_concurrent_streams` | Configuration |
| `HealthReporter` | `::new()` `.set_serving(bool)` `.is_serving()` | Health status |
| `GrpcMetrics` | `.record(method, duration_us, is_error)` `.to_prometheus()` `.method_count()` | Per-method metrics |

---

## vil_server_format

| Type | API | Description |
|------|-----|-------------|
| `FormatResponse<T>` | `::ok(data)` `::created(data)` `.with_headers(headers)` `.force_format(fmt)` `.with_status(code)` | Auto-negotiate JSON/Protobuf |
| `ResponseFormat` | `Json` `Protobuf` (feature-gated) `.content_type()` | Format enum |
| `negotiate()` | `negotiate(accept_header) → ResponseFormat` | Content negotiation |
| `is_supported()` | `is_supported(accept) → bool` | Check if format available |

---

## vil_mq_kafka

| Type | API | Description |
|------|-----|-------------|
| `KafkaConfig` | `::new(brokers)` `.group(id)` `.topic(t)` | Configuration + SASL |
| `KafkaProducer` | `::new(config)` `.publish(topic, payload)` `.publish_keyed(topic, key, payload)` `.messages_sent()` | Producer |
| `KafkaConsumer` | `::new(config)` `.start()` `.stop()` `.take_receiver()` `.inject_message()` `.is_running()` | Consumer with mpsc |
| `KafkaBridge` | `::new(target)` `.bridge(msg)` `.bridged_count()` | Kafka → Tri-Lane SHM |
| `KafkaMetrics` | `.to_prometheus()` | 4 atomic counters |

---

## vil_mq_mqtt

| Type | API | Description |
|------|-----|-------------|
| `MqttConfig` | `::new(url)` `.client_id(id)` `.qos(qos)` `.tls(bool)` | Configuration |
| `QoS` | `AtMostOnce` `AtLeastOnce` `ExactlyOnce` | Quality of Service |
| `MqttClient` | `::new(config)` `.publish(topic, payload, qos)` `.subscribe(filter)` `.disconnect()` `.is_connected()` | MQTT client |
| `MqttBridge` | `::new(target)` `.bridge(topic, payload)` `.bridged_count()` | MQTT → Tri-Lane SHM |

---

## vil_mq_nats

| Type | API | Description |
|------|-----|-------------|
| `NatsConfig` | `::new(url)` `.with_token(t)` `.with_userpass(u, p)` `.tls(bool)` `.name(n)` | Configuration |
| `NatsClient` | `::connect(config)` `.publish(subject, payload)` `.subscribe(subject)` `.request(subject, payload)` `.disconnect()` `.is_connected()` | Core pub/sub + request/reply |
| `NatsSubscription` | `.next()` `.subject()` | Subscription iterator |
| `NatsMessage` | `.subject` `.payload` `.reply_to` | Received message |
| `JetStreamClient` | `::new()` `.create_stream(config)` `.create_consumer(stream, config)` `.publish(subject, payload)` `.streams()` | Persistent streaming |
| `StreamConfig` | `name` `subjects` `retention` `max_msgs` `max_bytes` | Stream definition |
| `ConsumerConfig` | `durable_name` `filter_subject` `ack_policy` `deliver_policy` | Consumer definition |
| `JsMessage` | `.subject` `.payload` `.sequence` `.ack()` `.nack()` `.is_acked()` | JetStream message with ack |
| `KvStore` | `::new(bucket)` `.put(key, value)` `.get(key)` `.delete(key)` `.keys()` `.watch()` `.len()` | Distributed key-value |
| `KvEntry` | `.key` `.value` `.revision` | KV entry |
| `NatsBridge` | `::new(target)` `.bridge(subject, payload)` `.bridged_count()` | NATS → Tri-Lane SHM |
| `NatsMetrics` | `.to_prometheus()` | 7 atomic counters |

---

## AI Plugin System

### SseCollect (vil_server_core::sse_collect)

Built-in SSE stream collector with dialect support and async client.

| Method | Description |
|--------|-------------|
| `SseCollect::post_to(url)` | POST with built-in client (OpenAI default) |
| `SseCollect::get_from(url)` | GET with built-in client (W3C standard) |
| `SseCollect::post(client, url)` | POST with external client |
| `.dialect(SseDialect::openai())` | Apply provider preset |
| `.json_tap("path")` | Extract JSON field from SSE data |
| `.bearer_token(token)` | Authorization: Bearer |
| `.anthropic_key(key)` | x-api-key + anthropic-version |
| `.api_key_param(key)` | ?key= query parameter |
| `.done_marker("[END]")` | Custom data done marker |
| `.done_event("stop")` | Custom event done type |
| `.done_json_field("done", true)` | Custom JSON field done |
| `.body(json)` | Set request body |
| `.collect_text()` | Collect stream to String |

### VilPlugin Trait

| Method | Description |
|--------|-------------|
| `id() -> &str` | Unique plugin identifier |
| `version() -> &str` | Semantic version |
| `description() -> &str` | Human-readable description |
| `capabilities() -> Vec<PluginCapability>` | What plugin provides |
| `dependencies() -> Vec<PluginDependency>` | Required/optional deps |
| `register(&self, ctx: &mut PluginContext)` | Register services + resources |
| `health() -> PluginHealth` | Health check |

### vil_plugin_sdk (Stable Community Plugin Interface)

Single dependency for community plugin authors: `vil_plugin_sdk = "0.6"`

| Module | API | Description |
|--------|-----|-------------|
| `prelude` | `use vil_plugin_sdk::prelude::*;` | All plugin types in one import |
| `PluginBuilder` | `.new(id, version)` `.description(s)` `.capability(cap)` `.dependency(dep)` `.on_register(fn)` `.on_health(fn)` `.build()` | Ergonomic plugin construction |
| `PluginManifest` | `.new(name, version)` `.author(s)` `.license(s)` `.provides(cap)` `.requires(dep)` `.config_field(name, schema)` `.min_vil(version)` `.from_file(path)` `.to_json()` `.validate()` | Declarative plugin metadata |
| `ConfigFieldSchema` | `::string()` `::integer()` `::boolean()` `.required()` `.secret()` `.description(s)` `.default_value(json)` | Config field schema |
| `PluginTestHarness` | `.new()` `.register(plugin)` `.register_all(plugins)` `.service_count()` `.service_names()` `.has_resource::<T>(name)` `.get_resource::<T>(name)` `.route_count()` `.routes()` | Unit test harness |

### ServiceProcess Semantic Declarations

| Method | Description |
|--------|-------------|
| `.emits::<T>()` | Declare emitted AI event type (Data Lane) |
| `.faults::<T>()` | Declare fault type (Control Lane) |
| `.manages::<T>()` | Declare managed state type (Data Lane) |

---

## vil_observer

Embedded monitoring dashboard and JSON API. Enable via `.observer(true)`.

### Observer API Endpoints

| Endpoint | Method | Response |
|----------|--------|----------|
| `/_vil/api/topology` | GET | `TopologyResponse { app_name, services[], uptime_secs, total_requests }` |
| `/_vil/api/metrics` | GET | `{ endpoints[], uptime_secs, total_requests }` |
| `/_vil/api/health` | GET | `{ status, timestamp }` |
| `/_vil/api/routes` | GET | `RouteInfo[] { method, path, exec_class, request_count, avg_latency_us, error_rate }` |
| `/_vil/api/shm` | GET | `ShmStats { configured_mb, ring_stripes, ring_total_capacity, ring_total_used, ring_total_drops }` |
| `/_vil/api/logs/recent` | GET | `LogEntry[]` |
| `/_vil/api/system` | GET | `SystemInfo { pid, uptime_secs, rust_version, vil_version, os, arch, cpu_count, memory_rss_kb, fd_count, thread_count }` |
| `/_vil/api/config` | GET | `{ profile, log_level, shm_size_mb }` |
| `/_vil/dashboard/` | GET | Embedded SPA dashboard (HTML) |

### Core Types

| Type | Description |
|------|-------------|
| `MetricsCollector` | Global metrics registry with atomic per-endpoint counters |
| `EndpointMetrics` | Per-endpoint: requests, errors, total/min/max latency (AtomicU64) |
| `EndpointSnapshot` | Serializable snapshot: path, method, requests, errors, error_rate, avg/min/max latency |

### Semantic Events (`vil_observer::events`)

| Event | Fields |
|-------|--------|
| `ObserverMetricsSnapshot` | `total_requests: u64, endpoint_count: u32, uptime_secs: u64, timestamp_ns: u64` |
| `ObserverDashboardAccess` | `client_hash: u32, path_hash: u32, timestamp_ns: u64` |
| `ObserverErrorAlert` | `endpoint_hash: u32, error_rate_bps: u32, request_count: u64, timestamp_ns: u64` |

---

*VIL Community — [github.com/OceanOS-id/VIL](https://github.com/OceanOS-id/VIL)*
