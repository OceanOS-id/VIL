# VIL VWFD Test Suite Report

**Date:** 2026-04-12
**Mode:** `VIL_MODE=vwfd`
**Total Examples:** 112
**Test Runner:** `vil-testsuite/run-examples.sh --vwfd`

---

## 1. Executive Summary

| Metric | Count |
|--------|-------|
| Total VWFD examples | 112 |
| Examples with test.sh | 109 |
| Custom Code Activity: NativeCode | 67 examples |
| Custom Code Activity: Function (WASM) | 21 examples |
| Custom Code Activity: Sidecar | 12 examples |
| Pure Workflow (Connector/Transform only) | 30 examples |
| Observer-enabled | 2 examples |
| VilORM + SQLite (real DB) | 5 examples |

### Activity Type Distribution (YAML level)

| Activity Type | Workflow Files |
|---------------|---------------|
| NativeCode | 114 |
| Function (WASM) | 21 |
| Sidecar | 14 |
| Connector | 67 |
| Transform | 1 |

### Programming Languages

| Category | Languages |
|----------|-----------|
| WASM | Rust, Python, Java, Go, C, AssemblyScript |
| Sidecar | Python, Node.js, Java, Go, Ruby, PHP, C#, Lua, R |
| All examples | Rust (main.rs host) |

---

## 2. Full Example Catalog

### Legend

- **Arch**: Architecture pattern — `GW` Gateway, `REST` REST API, `WS` WebSocket, `GQL` GraphQL, `SSE` Server-Sent Events, `Pipe` Pipeline, `MQ` Message Queue, `DB` Database, `Store` Storage, `Proto` Protocol, `Trig` Trigger
- **Pattern**: `Req-Res` Request-Response, `Pub-Sub` Publish-Subscribe, `Fan-Out` Fan-Out, `Fan-In` Fan-In, `Stream` Streaming, `CRUD` Create-Read-Update-Delete, `Sched` Scheduler, `HA` High Availability
- **Code**: Custom code activity — `N` NativeCode, `W` WASM Function, `S` Sidecar, `C` Connector (inline VilQuery/HTTP), `T` Transform, `—` none (pure workflow)
- **Lang**: WASM or Sidecar language
- **VIL Features**: `ORM` vil_orm, `SQLx` vil_db_sqlx, `Obs` observer, `VQ` VilQuery inline YAML, `Expr` v-cel expressions

---

### 0xx — Basic Examples

| # | Name | Arch | Pattern | Business Domain | Code | Lang | Workflows | VIL Features | Port |
|---|------|------|---------|-----------------|------|------|-----------|--------------|------|
| 001 | ai-gw-demo | GW | Req-Res | AI Gateway | C | — | 1 | VQ, Expr | 3101 |
| 001b | vilapp-ai-gw-benchmark | GW | Req-Res | AI Gateway Benchmark | C | — | 2 | VQ, Expr | 3082 |
| 002 | vilapp-gateway | GW | Req-Res | API Gateway | C | — | 1 | VQ, Expr | 3102 |
| 003 | hello-server | REST | Req-Res | Currency Exchange | W | Rust | 1 | WASM, Expr | 8080 |
| 004 | rest-crud | REST | CRUD | Task Manager | C | — | 6 | VQ, SQLx, Expr | 8080 |
| 005 | multiservice-mesh-ndjson | Pipe | Stream | Payload Validation | W | Rust | 1 | WASM, Expr | 3105 |
| 006 | shm-extractor | Pipe | Req-Res | Trade Processing | W | Go | 1 | WASM | 3106 |
| 007 | credit-npl-filter | REST | Req-Res | Credit NPL Filter | S+C | Ruby | 1 | Sidecar, Expr | 3107 |
| 008 | credit-quality-monitor | REST | Req-Res | Credit Quality | N+S | Java | 1 | Sidecar | 3108 |
| 009 | credit-regulatory-slik | REST | Req-Res | SLIK Regulatory | N+W | Java | 1 | WASM, Expr | 3109 |
| 010 | websocket-chat | WS | Pub-Sub | Chat System | N+S | Node.js | 2 | Sidecar | 8080 |
| 011 | graphql-api | GQL | Req-Res | Product Catalog | N+W | AssemblyScript | 3 | WASM, Expr | 8080 |
| 012 | plugin-database | REST | CRUD | Blog + Comments | N | — | 8 | — | 8080 |
| 013 | nats-worker | MQ | Pub-Sub | Task Queue | N | — | 4 | — | 8080 |
| 014 | kafka-stream | MQ | Stream | Event Stream | N | — | 4 | — | 8080 |
| 015 | mqtt-iot-gateway | MQ | Pub-Sub | IoT Telemetry | N | — | 4 | — | 8080 |
| 016 | ai-rag-gateway | GW | Req-Res | RAG Gateway | N | — | 1 | — | 3084 |
| 017 | production-fullstack | REST | CRUD | Fullstack App | N | — | 5 | — | 8080 |
| 018 | ai-multi-model-router | GW | Req-Res | Multi-Model AI | N | — | 3 | — | 8080 |
| 019 | ai-multi-model-advanced | GW | Req-Res | Advanced AI Router | N | — | 2 | — | 8080 |
| 020 | ai-ab-testing | GW | Req-Res | A/B Testing | N+W | Rust | 3 | WASM | 8080 |
| 021 | wasm-faas | REST | Req-Res | FaaS Pricing | W | Rust | 1 | WASM | 3121 |
| 022 | sidecar-python | REST | Req-Res | Python Integration | N | — | 2 | — | 8080 |
| 023 | hybrid-wasm-sidecar | REST | Req-Res | Hybrid Demo | N | — | 2 | — | 8080 |
| 024 | llm-chat | GW | Req-Res | LLM Chat | N | — | 1 | — | 3090 |
| 025 | rag-service | GW | Req-Res | RAG Service | N | — | 1 | — | 3091 |
| 026 | ai-agent | GW | Req-Res | AI Agent | N | — | 1 | — | 8080 |
| 027 | vilserver-minimal | REST | Req-Res | Minimal Server | N | — | 2 | Obs | 8080 |
| 028 | sse-hub-streaming | SSE | Stream | SSE Hub | N | — | 2 | — | 8080 |
| 029 | vil-handler-endpoint | REST | Req-Res | Handler Demo | N | — | 3 | — | 8080 |
| 030 | trilane-messaging | MQ | Pub-Sub | Tri-Lane Messaging | N | — | 1 | — | 8080 |
| 031 | mesh-routing | Pipe | Req-Res | Mesh Routing | N | — | 2 | — | 8080 |
| 032 | failover-ha | REST | HA | Payment HA Failover | N+S | C# | 4 | Sidecar | 8080 |
| 033 | shm-write-through | REST | Req-Res | SHM Write-Through | N | — | 2 | — | 8080 |
| 034 | blocking-task | REST | Req-Res | Credit Risk Monte Carlo | N+S | Python | 2 | Sidecar | 8080 |
| 035 | vil-service-module | REST | Req-Res | Hospital Appointments | N+S | Lua | 2 | Sidecar | 8080 |
| 036 | sse-event-builder | SSE | Stream | SSE Event Builder | N | — | 2 | — | 8080 |
| 037 | vilmodel-derive | REST | Req-Res | Insurance Claims | N+S | PHP | 2 | Sidecar | 8080 |
| 038 | vil-app-dsl | REST | CRUD | Restaurant Orders | N+W | AssemblyScript | 4 | WASM | 8080 |
| 039 | observer-dashboard | REST | Req-Res | Observer Dashboard | N | — | 2 | Obs | 8080 |
| 040 | auth-middleware-stack | REST | Req-Res | Auth Middleware | N | — | 3 | — | 8080 |
| 041 | sidecar-failover | REST | Req-Res | ML Scoring | S | Python | 2 | Sidecar | 8080 |
| 042 | scripting-sandbox | REST | Req-Res | Dynamic Pricing | N+W | AssemblyScript | 3 | WASM | 8080 |
| 043 | integration-test | REST | Req-Res | Integration Test | N | — | 3 | — | 8080 |
| 044 | graphql-subscriptions | GQL | Pub-Sub | Notifications | N+S | Node.js | 2 | Sidecar | 8080 |
| 045 | exec-class-pinned | REST | Req-Res | IoT Sensor FFT | N+W | C | 2 | WASM | 8080 |
| 046 | mesh-scatter-gather | Pipe | Fan-Out | Scatter-Gather | N | — | 1 | — | 8080 |
| 047 | custom-error-stack | REST | Req-Res | Banking Transfer | N+W | Java | 2 | WASM | 8080 |

### 1xx — Pipeline Examples

| # | Name | Arch | Pattern | Business Domain | Code | Lang | Workflows | VIL Features | Port |
|---|------|------|---------|-----------------|------|------|-----------|--------------|------|
| 101 | 3node-transform-chain | Pipe | Stream | Transform Chain | N | — | 1 | — | 3203 |
| 101b | multi-pipeline-benchmark | Pipe | Stream | Pipeline Bench | N | — | 1 | — | 3201 |
| 101c | vilapp-multi-pipeline | Pipe | Stream | VilApp Pipeline | N | — | 1 | — | 3202 |
| 102 | fanout-scatter | Pipe | Fan-Out | NPL Fanout | W | Go | 1 | WASM | 3302 |
| 103 | fanin-gather | Pipe | Fan-In | Aggregation | S | Go | 1 | Sidecar | 3303 |
| 104 | diamond-topology | Pipe | Fan-Out | Diamond DAG | N | — | 1 | — | 3206 |
| 105 | multi-workflow | Pipe | Req-Res | Multi-Workflow | N | — | 1 | — | 3207 |
| 106 | sse-standard-dialect | SSE | Stream | SSE Standard | N | — | 1 | — | 3208 |
| 107 | process-traced | Pipe | Stream | Supply Chain | W | Go | 1 | WASM | 3307 |
| 108 | dag-scheduler | Pipe | Sched | DAG Scheduler | N | — | 1 | — | 8080 |

### 2xx — LLM Examples

| # | Name | Arch | Pattern | Business Domain | Code | Lang | Workflows | VIL Features | Port |
|---|------|------|---------|-----------------|------|------|-----------|--------------|------|
| 201 | basic-chat | GW | Req-Res | LLM Chat | C | — | 1 | VQ, Expr | 3100 |
| 202 | multi-model-routing | GW | Req-Res | Model Routing | C | — | 1 | VQ, Expr | 8080 |
| 203 | code-review-with-tools | GW | Req-Res | Code Review AI | N+C | — | 1 | VQ, Expr | 3102 |
| 204 | streaming-translator | GW | Stream | Streaming Translation | C | — | 1 | VQ, Expr | 3103 |
| 205 | chunked-summarizer | GW | Stream | Document Summary | N+C | — | 1 | VQ, Expr | 3104 |
| 206 | decision-routing | GW | Req-Res | Decision Routing | C | — | 1 | VQ, Expr | 8080 |

### 3xx — RAG Examples

| # | Name | Arch | Pattern | Business Domain | Code | Lang | Workflows | VIL Features | Port |
|---|------|------|---------|-----------------|------|------|-----------|--------------|------|
| 301 | basic-vector-search | GW | Req-Res | Vector Search | W+C | Python | 1 | WASM, VQ | 3110 |
| 302 | multi-source-fanin | Pipe | Fan-In | Multi-Source RAG | W+C | Python | 1 | WASM, VQ | 3111 |
| 303 | hybrid-exact-semantic | GW | Req-Res | Hybrid Search | W+C | Python | 1 | WASM, VQ | 3112 |
| 304 | citation-extraction | GW | Req-Res | Citation Extraction | N+S+C | R | 1 | Sidecar, VQ | 3113 |
| 305 | guardrail-pipeline | Pipe | Stream | PII Guardrail | N+W+C | Python | 1 | WASM, VQ | 3114 |
| 306 | ai-event-tracking | GW | Req-Res | Event Tracking | W+C | Python | 1 | WASM, VQ | 8080 |
| 307 | vectordb-knowledge-index | DB | Req-Res | Knowledge Index | N | — | 1 | — | 3107 |
| 308 | full-pipeline-ingest-query | Pipe | Stream | Full RAG Pipeline | W+C | Python | 1 | WASM, VQ | 8080 |

### 4xx — Agent Examples

| # | Name | Arch | Pattern | Business Domain | Code | Lang | Workflows | VIL Features | Port |
|---|------|------|---------|-----------------|------|------|-----------|--------------|------|
| 401 | calculator | GW | Req-Res | Calculator Agent | C | — | 1 | VQ, Expr | 3120 |
| 402 | http-researcher | GW | Req-Res | HTTP Research Agent | N+C | — | 1 | VQ, Expr | 8080 |
| 403 | code-file-reviewer | GW | Req-Res | Code Review Agent | N+C | — | 1 | VQ, Expr | 3122 |
| 404 | data-csv-analyst | GW | Req-Res | CSV Analyst | N+W+C | C | 1 | WASM, VQ | 3123 |
| 405 | react-multi-tool | GW | Req-Res | ReAct Multi-Tool | N+C | — | 1 | VQ, Expr | 3124 |
| 406 | vil-handler-shm | REST | Req-Res | SHM Handler Agent | N+S | Python | 1 | Sidecar | 3126 |
| 407 | multi-agent-orchestration | Pipe | Fan-Out | Multi-Agent | N | — | 1 | — | 8080 |

### 5xx — VilLog / Observability Examples

| # | Name | Arch | Pattern | Business Domain | Code | Lang | Workflows | VIL Features | Port |
|---|------|------|---------|-----------------|------|------|-----------|--------------|------|
| 501 | stdout-dev | REST | Req-Res | Log: stdout | N | — | 1 | VilLog | 3232 |
| 502 | file-rolling | REST | Req-Res | Log: File Rolling | N | — | 1 | VilLog | 3233 |
| 503 | multi-drain | REST | Req-Res | Log: Multi Drain | N | — | 1 | VilLog | 3234 |
| 504 | benchmark-comparison | REST | Req-Res | Log: Benchmark | N | — | 1 | VilLog | 3235 |
| 505 | tracing-bridge | REST | Req-Res | Log: Tracing Bridge | N | — | 1 | VilLog | 3236 |
| 506 | structured-events | REST | Req-Res | Log: Structured | N | — | 1 | VilLog | 3237 |
| 507 | bench-file-drain | REST | Req-Res | Log: File Bench | N | — | 1 | VilLog | 3238 |
| 508 | bench-multithread | REST | Req-Res | Log: MT Bench | N | — | 1 | VilLog | 3239 |
| 509 | phase1-integration | REST | Req-Res | Log: Integration | N | — | 1 | VilLog | 3240 |

### 6xx — Database / Storage Examples

| # | Name | Arch | Pattern | Business Domain | Code | Lang | Workflows | VIL Features | Port |
|---|------|------|---------|-----------------|------|------|-----------|--------------|------|
| 601 | storage-s3-basic | Store | Req-Res | S3 Storage | C | — | 1 | VQ | 3241 |
| 602 | db-mongo-crud | DB | CRUD | MongoDB CRUD | C | — | 1 | VQ | 3242 |
| 603 | db-clickhouse-batch | DB | Stream | ClickHouse Batch | C | — | 1 | VQ | 3243 |
| 604 | db-elastic-search | DB | Req-Res | Elasticsearch | C | — | 1 | VQ | 3244 |
| 605 | db-vilorm-crud | DB | CRUD | Blog Platform | N+C | — | 9 | ORM, SQLx, VQ | 8080 |
| 606 | db-vilorm-ecommerce | DB | CRUD | E-Commerce | N+C | — | 7 | ORM, SQLx, VQ | 8086 |
| 607 | db-vilorm-multitenant | DB | CRUD | Multi-Tenant | C | — | 1 | VQ | 8087 |
| 608 | db-vilorm-analytics | DB | Req-Res | Analytics | C | — | 1 | VQ | 8088 |
| 609 | db-vilorm-overhead-bench | DB | Req-Res | ORM Benchmark | C | — | 1 | VQ | 3249 |
| 610 | storage-multi-cloud | Store | Req-Res | Multi-Cloud Storage | C | — | 1 | VQ | 8080 |
| 611 | db-timeseries-iot | DB | Stream | TimeSeries IoT | C | — | 1 | VQ | 8080 |

### 7xx — Message Queue / Protocol Examples

| # | Name | Arch | Pattern | Business Domain | Code | Lang | Workflows | VIL Features | Port |
|---|------|------|---------|-----------------|------|------|-----------|--------------|------|
| 701 | mq-rabbitmq-pubsub | MQ | Pub-Sub | RabbitMQ | C | — | 1 | VQ | 3252 |
| 702 | mq-sqs-send-receive | MQ | Req-Res | AWS SQS | C | — | 1 | VQ | 3253 |
| 703 | protocol-soap-client | Proto | Req-Res | SOAP Client | C | — | 1 | VQ | 3254 |
| 704 | protocol-modbus-read | Proto | Req-Res | Modbus IoT | C | — | 1 | VQ | 3255 |
| 705 | protocol-grpc-gateway | Proto | Req-Res | gRPC Gateway | W | Java | 1 | WASM | 3705 |
| 706 | mq-pulsar-messaging | MQ | Pub-Sub | Apache Pulsar | C | — | 1 | VQ | 8080 |

### 8xx — Trigger Examples

| # | Name | Arch | Pattern | Business Domain | Code | Lang | Workflows | VIL Features | Port |
|---|------|------|---------|-----------------|------|------|-----------|--------------|------|
| 801 | trigger-cron-basic | Trig | Sched | Cron Job | C | — | 1 | VQ | 3258 |
| 802 | trigger-fs-watcher | Trig | Stream | Filesystem Watch | C | — | 1 | VQ | 3259 |
| 803 | trigger-webhook-receiver | Trig | Req-Res | Webhook Receiver | C | — | 1 | VQ | 3260 |
| 804 | trigger-cdc-postgres | Trig | Stream | CDC Postgres | C | — | 1 | VQ | 3261 |
| 805 | trigger-email | Trig | Req-Res | Email Trigger | C | — | 1 | VQ | 3262 |
| 806 | trigger-iot | Trig | Stream | IoT Trigger | C | — | 1 | VQ | 3263 |
| 807 | trigger-evm-blockchain | Trig | Stream | EVM Blockchain | C | — | 1 | VQ | 3264 |

---

## 3. WASM Module Inventory

| # | Example | Source Language | Source File | WASM Binary | Function |
|---|---------|---------------|-------------|-------------|----------|
| 003 | hello-server | Rust | `wasm/rust/currency_convert.rs` | `currency_convert.wasm` | execute |
| 005 | multiservice-mesh | Rust | `wasm/rust/validate_payload.rs` | — | execute |
| 006 | shm-extractor | Go | `wasm/go/process_trade.go` | — | main |
| 009 | credit-regulatory-slik | Java | `wasm/java/SlikReportFormatter.java` | — | main |
| 011 | graphql-api | AssemblyScript | `wasm/assemblyscript/products.ts` | `products.wasm` | query |
| 020 | ai-ab-testing | Rust | `wasm/rust/deterministic_split.rs` | `deterministic_split.wasm` | main |
| 021 | wasm-faas | Rust | (external) | `modules/pricing.wasm` | — |
| 038 | vil-app-dsl | AssemblyScript | `wasm/assemblyscript/restaurant.ts` | `restaurant.wasm` | processOrder |
| 042 | scripting-sandbox | AssemblyScript | `wasm/assemblyscript/pricing.ts` | `pricing.wasm` | calculate |
| 045 | exec-class-pinned | C | `wasm/c/iot_fft.c` | `iot_fft.wasm` | main |
| 047 | custom-error-stack | Java | `wasm/java/BankingTransfer.java` | `BankingTransfer.wasm` | main |
| 102 | fanout-scatter | Go | `wasm/go/fanout_npl.go` | — | main |
| 107 | process-traced | Go | `wasm/go/supply_chain.go` | — | main |
| 301 | rag-basic-vector-search | Python | `wasm/python/rag_embed_and_search.py` | — | — |
| 302 | rag-multi-source-fanin | Python | `wasm/python/rag_multi_source.py` | — | — |
| 303 | rag-hybrid-exact-semantic | Python | `wasm/python/rag_hybrid_search.py` | — | — |
| 305 | rag-guardrail-pipeline | Python | `wasm/python/guardrail_pii_detector.py` | — | — |
| 306 | rag-ai-event-tracking | Python | `wasm/python/rag_keyword_search.py` | — | — |
| 308 | rag-full-pipeline | Python | `wasm/python/rag_embed_search.py` | — | — |
| 404 | data-csv-analyst | C | `wasm/c/csv_stats.c` | — | main |
| 705 | protocol-grpc-gateway | Java | `wasm/java/PaymentProcessor.java` | — | main |

**WASM Languages:** 6 (Rust: 3, Python: 6, Java: 3, Go: 3, C: 2, AssemblyScript: 3)
**Compiled .wasm binaries:** 7 (003, 011, 020, 038, 042, 045, 047)

---

## 4. Sidecar Script Inventory

| # | Example | Language | Runtime | Script Path |
|---|---------|----------|---------|-------------|
| 007 | credit-npl-filter | Ruby | `ruby` | `sidecar/ruby/npl_flag.rb` |
| 008 | credit-quality-monitor | Java | `java` | `sidecar/java/CreditSchemaValidator.java` |
| 010 | websocket-chat | Node.js | `node` | `sidecar/nodejs/chat_processor.js` |
| 032 | failover-ha | C# | `dotnet-script` | `sidecar/csharp/PaymentHA.cs` |
| 034 | blocking-task | Python | `python3` | `sidecar/python/monte_carlo_risk.py` |
| 035 | vil-service-module | Lua | `lua5.4` | `sidecar/lua/scheduler.lua` |
| 037 | vilmodel-derive | PHP | `php` | `sidecar/php/claim_processor.php` |
| 041 | sidecar-failover | Python | `python3` | `sidecar/python/ml_scorer.py` |
| 044 | graphql-subscriptions | Node.js | `node` | `sidecar/nodejs/notification_builder.js` |
| 103 | fanin-gather | Go | `go run` | `sidecar/go/fanin_aggregator.go` |
| 304 | rag-citation-extraction | R | `Rscript` | `sidecar/r/citation_extractor.R` |
| 406 | agent-vil-handler-shm | Python | `python3` | `sidecar/python/velocity_checker.py` |

**Sidecar Languages:** 9 (Python: 3, Node.js: 2, Java: 1, Go: 1, Ruby: 1, PHP: 1, C#: 1, Lua: 1, R: 1)

---

## 5. Test Results — 11 Converted Examples (2026-04-12)

### WASM Function Activity Tests

| # | Example | WASM Lang | Assertions | Result |
|---|---------|-----------|------------|--------|
| 011 | graphql-api | AssemblyScript | 7/7 | PASS |
| 020 | ai-ab-testing | Rust | 7/7 | PASS |
| 038 | vil-app-dsl | AssemblyScript | 10/10 | PASS |
| 042 | scripting-sandbox | AssemblyScript | 5/5 | PASS |
| 045 | exec-class-pinned | C | 2/2 | PASS |
| 047 | custom-error-stack | Java | 4/4 | PASS |

### Sidecar Activity Tests

| # | Example | Sidecar Lang | Assertions | Result |
|---|---------|-------------|------------|--------|
| 032 | failover-ha | C# | 7/7 | PASS |
| 034 | blocking-task | Python | 6/6 | PASS |
| 035 | vil-service-module | Lua | 4/4 | PASS |
| 037 | vilmodel-derive | PHP | 7/7 | PASS |
| 044 | graphql-subscriptions | Node.js | 3/3 | PASS |

**Total: 11/11 examples PASS, 62/62 assertions PASS**

---

## 6. Architecture Pattern Distribution

| Pattern | Count | Examples |
|---------|-------|----------|
| Request-Response | 62 | Most 0xx, 2xx, 3xx, 4xx, 5xx, 6xx |
| CRUD | 10 | 004, 012, 017, 605, 606, 607, 602 |
| Streaming / SSE | 10 | 005, 028, 036, 101, 106, 204, 603, 611, 804, 807 |
| Pub-Sub | 8 | 010, 013, 030, 044, 701, 706, 014, 015 |
| Fan-Out | 4 | 046, 102, 104, 407 |
| Fan-In | 2 | 103, 302 |
| High Availability | 1 | 032 |
| Scheduler | 2 | 108, 801 |
| Gateway / Proxy | 12 | 001, 001b, 002, 016, 018, 019, 201–206 |

## 7. Business Domain Distribution

| Domain | Count | Examples |
|--------|-------|----------|
| AI / LLM / RAG | 22 | 001, 001b, 016, 018–020, 024–026, 201–206, 301–308 |
| Financial / Banking | 7 | 007–009, 032, 034, 047, 609 |
| E-Commerce / Retail | 4 | 038, 042, 606, 705 |
| IoT / Sensor | 4 | 015, 045, 611, 806 |
| Healthcare | 1 | 035 |
| Insurance | 1 | 037 |
| Chat / Social | 3 | 010, 044, 030 |
| DevOps / Observability | 11 | 027, 039, 501–509 |
| Database / Storage | 11 | 601–611 |
| Message Queue | 6 | 013, 014, 701, 702, 706, 030 |
| Protocol | 4 | 703, 704, 705, 806 |
| Pipeline / Workflow | 10 | 101–108, 046, 407 |
| Agent / Tool Use | 7 | 401–407 |
| Trigger | 7 | 801–807 |
| General / Demo | 14 | 002–006, 017, 021–023, 027–029, 031, 033 |

---

## 8. VIL Feature Coverage

| VIL Feature | Examples Using It |
|-------------|-------------------|
| **WASM (Function)** | 003, 005, 006, 009, 011, 020, 021, 038, 042, 045, 047, 102, 107, 301–303, 305, 306, 308, 404, 705 |
| **Sidecar** | 007, 008, 010, 032, 034, 035, 037, 041, 044, 103, 304, 406 |
| **VilQuery inline YAML** | 001, 001b, 002, 004, 005, 007, 009, 201–206, 301–306, 308, 401–405, 601–611, 701–706, 801–807 |
| **vil_orm** | 605, 606 |
| **vil_db_sqlx** | 004, 605, 606 |
| **Observer** | 027, 039 |
| **VilLog** | 501–509 |
| **v-cel Expressions** | All workflow YAMLs with `language: v-cel` |
| **Multi-Workflow** | 004(6), 010(2), 011(3), 012(8), 013(4), 014(4), 015(4), 017(5), 018(3), 020(3), 029(3), 032(4), 038(4), 042(3), 043(3), 605(9), 606(7) |

---

## 9. Runtime Dependencies

| Runtime | Version | Required By |
|---------|---------|-------------|
| Rust (host) | stable | All 112 examples |
| wasmtime | 24.x | WASM examples (feature `wasm`) |
| python3 | 3.x | 034, 041, 406 |
| node | 18+ | 010, 044 |
| php | 8.x | 037 |
| lua5.4 | 5.4.x | 035 |
| dotnet-script | 2.0 | 032 |
| ruby | 3.x | 007 |
| java | 21+ | 008 |
| go | 1.21+ | 103 |
| Rscript | 4.x | 304 |
| SQLite | (embedded) | 004, 605, 606, 607, 608, 609 |

---

## 10. Load Test & Benchmark (2026-04-12)

**Environment:** Linux 6.8.0, Rust release build, single machine localhost
**Tools:** `curl` (latency), `hey` -n 1000 -c 50, `vastar` -n 1000 -c 50

### NativeCode Baseline

| # | Example | Endpoint | curl latency | hey req/s | vastar req/s |
|---|---------|----------|-------------|-----------|--------------|
| 011 | graphql-api | GET /api/graphql/schema | 0.3 ms | 45,506 | 80,747 |
| 020 | ai-ab-testing | GET /api/ab/metrics | 0.4 ms | 53,043 | 114,938 |
| 038 | vil-app-dsl | GET /api/.../menu | 0.4 ms | 54,671 | 93,147 |
| 042 | scripting-sandbox | GET /api/pricing/rules | 0.4 ms | 53,549 | 112,798 |
| 045 | exec-class-pinned | GET /api/sensor/.../stats | 0.6 ms | 54,647 | 110,368 |
| 047 | custom-error-stack | GET /api/banking/accounts | 0.3 ms | 57,818 | 106,278 |
| 032 | failover-ha | GET /api/primary/health | 0.4 ms | 34,876 | 123,295 |
| 034 | blocking-task | GET /api/.../risk/health | 0.4 ms | 27,193 | 113,224 |
| 037 | vilmodel-derive | GET /api/claims/.../sample | 0.4 ms | 50,775 | 108,829 |
| 044 | graphql-subs | GET /api/.../stats | 0.3 ms | 53,367 | 122,195 |
| 035 | vil-service-module | POST /api/.../register | 0.3 ms | 41,889 | 87,439 |

**NativeCode average:** ~0.4 ms latency, ~47K hey req/s, ~107K vastar req/s

### WASM Function Activity Benchmark

| # | Example | WASM Lang | Endpoint | curl latency | hey req/s | vastar req/s |
|---|---------|-----------|----------|-------------|-----------|--------------|
| 011 | graphql-api | AssemblyScript | POST /api/graphql/query | 19.5 ms | 73 | 80 |
| 020 | ai-ab-testing | Rust | POST /api/ab/infer | 16.0 ms | 83 | 87 |
| 038 | vil-app-dsl | AssemblyScript | POST /api/.../order | 20.9 ms | 67 | 75 |
| 042 | scripting-sandbox | AssemblyScript | POST /api/pricing/calculate | 22.7 ms | 63 | 65 |
| 045 | exec-class-pinned | C | POST /api/sensor/.../process | 24.1 ms | 60 | 60 |
| 047 | custom-error-stack | Java | POST /api/banking/transfer | 24.5 ms | 63 | 62 |

**WASM average:** ~21 ms latency, ~68 hey req/s, ~72 vastar req/s
**Note:** WASM uses process-spawn model (wasmtime per-request), not pooled. Latency dominated by WASI module instantiation.

### Sidecar Activity Benchmark

| # | Example | Sidecar Lang | Endpoint | curl latency | hey req/s | vastar req/s |
|---|---------|-------------|----------|-------------|-----------|--------------|
| 035 | vil-service-module | **Lua** | POST /api/.../schedule | 1.9 ms | 4,592 | 5,981 |
| 037 | vilmodel-derive | **PHP** | POST /api/.../submit | 7.5 ms | 770 | 833 |
| 034 | blocking-task | **Python** | POST /api/.../assess | 20.0 ms | 271 | 326 |
| 044 | graphql-subs | **Node.js** | POST /api/.../publish | 20.7 ms | 279 | 310 |
| 032 | failover-ha | **C#** | POST /api/primary/charge | 290.6 ms | 14 | 15 |

**Sidecar ranking by throughput (vastar):**
1. Lua — **5,981 req/s** (1.9 ms, lightweight interpreter)
2. PHP — **833 req/s** (7.5 ms, PHP-CLI fast startup)
3. Python — **326 req/s** (20 ms, doing actual Monte Carlo simulation)
4. Node.js — **310 req/s** (21 ms, readline-based stdin)
5. C# — **15 req/s** (291 ms, dotnet-script JIT cold start per invocation)

### Throughput Comparison Chart

```
Activity Type         vastar req/s (avg)
─────────────────────────────────────────
NativeCode            ███████████████████████████████████████████████████████  107,000
Sidecar Lua           ███                                                       5,981
Sidecar PHP           ▌                                                           833
Sidecar Python        ▏                                                           326
Sidecar Node.js       ▏                                                           310
WASM (all langs)      ▏                                                            72
Sidecar C#            ▏                                                            15
```

### Analysis

| Metric | NativeCode | WASM | Sidecar (median) |
|--------|-----------|------|-----------------|
| Avg latency | 0.4 ms | 21 ms | 20 ms |
| Avg throughput (hey) | 47K req/s | 68 req/s | 279 req/s |
| Avg throughput (vastar) | 107K req/s | 72 req/s | 326 req/s |
| Overhead vs NativeCode | 1x | ~525x | ~330x |

**Key findings:**
- NativeCode is the fastest path — sub-millisecond, 100K+ req/s
- WASM overhead is dominated by wasmtime module instantiation (~15-25ms per request). A module pool would dramatically improve this.
- Sidecar performance varies 400x across languages (Lua 5,981 vs C# 15 req/s), driven by interpreter cold-start time
- Lua is the fastest sidecar runtime — nearly 6K req/s with <2ms latency
- C# via dotnet-script has severe cold-start penalty (~291ms) — a pre-compiled binary would be much faster
- vastar consistently measures 1.5-2x higher throughput than hey due to zero-copy HTTP client

---

*Generated: 2026-04-12 | VIL VWFD Mode | 112 examples, 189 workflow files*
