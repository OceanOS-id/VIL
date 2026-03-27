//! Built-in node type registry.
//!
//! Maps YAML `type:` values to crate names, categories, descriptions,
//! and default port layouts. Used by codegen, viz, and `vil node list`.

/// A registered built-in node type.
pub struct NodeTypeEntry {
    pub type_name: &'static str,
    pub crate_name: &'static str,
    pub category: &'static str,
    pub color: &'static str,
    pub description: &'static str,
    pub default_ports: &'static [(&'static str, &'static str, &'static str)], // (name, direction, lane)
}

/// All built-in node types — topology nodes + AI nodes + DB/MQ nodes.
pub const NODE_TYPES: &[NodeTypeEntry] = &[
    // ═══════════════════════════════════════════════════════════════════════
    // Core topology nodes
    // ═══════════════════════════════════════════════════════════════════════
    NodeTypeEntry {
        type_name: "http-sink",
        crate_name: "vil_new_http",
        category: "trigger",
        color: "#FFA000",
        description: "HTTP webhook listener — receives inbound requests",
        default_ports: &[
            ("trigger_out", "out", "trigger"),
            ("data_in", "in", "data"),
            ("ctrl_in", "in", "control"),
        ],
    },
    NodeTypeEntry {
        type_name: "http-source",
        crate_name: "vil_new_http",
        category: "source",
        color: "#00897B",
        description: "HTTP/SSE upstream caller — calls external APIs",
        default_ports: &[
            ("trigger_in", "in", "trigger"),
            ("data_out", "out", "data"),
            ("ctrl_out", "out", "control"),
        ],
    },
    NodeTypeEntry {
        type_name: "transform",
        crate_name: "vil_sdk",
        category: "transform",
        color: "#7B1FA2",
        description: "Processing node with custom logic (expr/handler/script/wasm)",
        default_ports: &[("in", "in", "data"), ("out", "out", "data")],
    },
    // ═══════════════════════════════════════════════════════════════════════
    // Tier 1: Core LLM
    // ═══════════════════════════════════════════════════════════════════════
    NodeTypeEntry {
        type_name: "llm_chat",
        crate_name: "vil_llm",
        category: "ai",
        color: "#7B1FA2",
        description: "LLM chat completion (OpenAI, Anthropic, Ollama)",
        default_ports: &[
            ("in", "in", "trigger"),
            ("out", "out", "data"),
            ("ctrl", "out", "control"),
        ],
    },
    NodeTypeEntry {
        type_name: "llm_embed",
        crate_name: "vil_embedder",
        category: "ai",
        color: "#7B1FA2",
        description: "Text embedding generation (batch or single)",
        default_ports: &[("in", "in", "trigger"), ("out", "out", "data")],
    },
    NodeTypeEntry {
        type_name: "rag_query",
        crate_name: "vil_rag",
        category: "ai",
        color: "#7B1FA2",
        description: "RAG query — retrieve + generate with context",
        default_ports: &[("in", "in", "trigger"), ("out", "out", "data")],
    },
    NodeTypeEntry {
        type_name: "rag_ingest",
        crate_name: "vil_rag",
        category: "ai",
        color: "#7B1FA2",
        description: "RAG document ingestion — chunk + embed + store",
        default_ports: &[("in", "in", "trigger"), ("out", "out", "data")],
    },
    NodeTypeEntry {
        type_name: "rerank",
        crate_name: "vil_reranker",
        category: "ai",
        color: "#7B1FA2",
        description: "Result reranking (keyword, cross-encoder, RRF fusion)",
        default_ports: &[("in", "in", "data"), ("out", "out", "data")],
    },
    NodeTypeEntry {
        type_name: "chunk",
        crate_name: "vil_chunker",
        category: "ai",
        color: "#7B1FA2",
        description: "Document chunking (sentence, sliding, code, table)",
        default_ports: &[("in", "in", "data"), ("out", "out", "data")],
    },
    // ═══════════════════════════════════════════════════════════════════════
    // Tier 2: Agent
    // ═══════════════════════════════════════════════════════════════════════
    NodeTypeEntry {
        type_name: "agent",
        crate_name: "vil_agent",
        category: "agent",
        color: "#6A1B9A",
        description: "ReAct agent with tool calling (calculator, fetch, retrieval)",
        default_ports: &[
            ("in", "in", "trigger"),
            ("out", "out", "data"),
            ("ctrl", "out", "control"),
        ],
    },
    NodeTypeEntry {
        type_name: "agent_graph",
        crate_name: "vil_multi_agent",
        category: "agent",
        color: "#6A1B9A",
        description: "Multi-agent DAG orchestration",
        default_ports: &[("in", "in", "trigger"), ("out", "out", "data")],
    },
    NodeTypeEntry {
        type_name: "memory",
        crate_name: "vil_memory_graph",
        category: "agent",
        color: "#6A1B9A",
        description: "Persistent knowledge graph for agent long-term memory",
        default_ports: &[("in", "in", "data"), ("out", "out", "data")],
    },
    // ═══════════════════════════════════════════════════════════════════════
    // Tier 3: Knowledge & Retrieval
    // ═══════════════════════════════════════════════════════════════════════
    NodeTypeEntry {
        type_name: "graphrag",
        crate_name: "vil_graphrag",
        category: "knowledge",
        color: "#00897B",
        description: "Graph-enhanced RAG with entity extraction",
        default_ports: &[("in", "in", "data"), ("out", "out", "data")],
    },
    NodeTypeEntry {
        type_name: "context_optimize",
        crate_name: "vil_context_optimizer",
        category: "knowledge",
        color: "#00897B",
        description: "Context window optimization (token budget, dedup)",
        default_ports: &[("in", "in", "data"), ("out", "out", "data")],
    },
    NodeTypeEntry {
        type_name: "crawler",
        crate_name: "vil_crawler",
        category: "knowledge",
        color: "#00897B",
        description: "Web crawler (BFS, robots.txt, concurrency control)",
        default_ports: &[("in", "in", "trigger"), ("out", "out", "data")],
    },
    NodeTypeEntry {
        type_name: "doc_extract",
        crate_name: "vil_doc_extract",
        category: "knowledge",
        color: "#00897B",
        description: "Rule-based field extraction (invoice, receipt, resume)",
        default_ports: &[("in", "in", "data"), ("out", "out", "data")],
    },
    // ═══════════════════════════════════════════════════════════════════════
    // Tier 4: Safety & Optimization
    // ═══════════════════════════════════════════════════════════════════════
    NodeTypeEntry {
        type_name: "shield",
        crate_name: "vil_prompt_shield",
        category: "safety",
        color: "#C62828",
        description: "Prompt injection detection (Aho-Corasick patterns)",
        default_ports: &[
            ("in", "in", "data"),
            ("clean_out", "out", "data"),
            ("flagged_out", "out", "data"),
        ],
    },
    NodeTypeEntry {
        type_name: "prompt_optimize",
        crate_name: "vil_prompt_optimizer",
        category: "safety",
        color: "#C62828",
        description: "Auto-prompt optimization (grid/random search, evaluation)",
        default_ports: &[("in", "in", "data"), ("out", "out", "data")],
    },
    NodeTypeEntry {
        type_name: "speculative",
        crate_name: "vil_speculative",
        category: "inference",
        color: "#AD1457",
        description: "Speculative decoding (2-3x faster via draft+verify)",
        default_ports: &[("in", "in", "trigger"), ("out", "out", "data")],
    },
    // ═══════════════════════════════════════════════════════════════════════
    // Tier 5: Observability & Cost
    // ═══════════════════════════════════════════════════════════════════════
    NodeTypeEntry {
        type_name: "tracer",
        crate_name: "vil_ai_trace",
        category: "observability",
        color: "#546E7A",
        description: "AI distributed tracing (spans, operations, export)",
        default_ports: &[("in", "in", "data")],
    },
    NodeTypeEntry {
        type_name: "cost_tracker",
        crate_name: "vil_cost_tracker",
        category: "observability",
        color: "#546E7A",
        description: "AI cost tracking with budget enforcement",
        default_ports: &[("in", "in", "data"), ("alert", "out", "control")],
    },
    NodeTypeEntry {
        type_name: "ab_test",
        crate_name: "vil_ab_test",
        category: "observability",
        color: "#546E7A",
        description: "A/B testing framework (variant assignment, z-test)",
        default_ports: &[("in", "in", "trigger"), ("out", "out", "data")],
    },
    NodeTypeEntry {
        type_name: "model_registry",
        crate_name: "vil_model_registry",
        category: "observability",
        color: "#546E7A",
        description: "Model version management (register, promote, rollback)",
        default_ports: &[("in", "in", "trigger"), ("out", "out", "data")],
    },
    // ═══════════════════════════════════════════════════════════════════════
    // Tier 6: Multimodal & Specialized
    // ═══════════════════════════════════════════════════════════════════════
    NodeTypeEntry {
        type_name: "vision",
        crate_name: "vil_vision",
        category: "multimodal",
        color: "#00838F",
        description: "Image analysis (object detection, OCR, embedding)",
        default_ports: &[("in", "in", "data"), ("out", "out", "data")],
    },
    NodeTypeEntry {
        type_name: "transcriber",
        crate_name: "vil_audio",
        category: "multimodal",
        color: "#00838F",
        description: "Speech-to-text transcription",
        default_ports: &[("in", "in", "data"), ("out", "out", "data")],
    },
    NodeTypeEntry {
        type_name: "sql_agent",
        crate_name: "vil_sql_agent",
        category: "data",
        color: "#1565C0",
        description: "Text-to-SQL with injection prevention",
        default_ports: &[("in", "in", "trigger"), ("out", "out", "data")],
    },
    NodeTypeEntry {
        type_name: "data_prep",
        crate_name: "vil_data_prep",
        category: "data",
        color: "#1565C0",
        description: "Fine-tuning data pipeline (dedup, filter, format)",
        default_ports: &[("in", "in", "data"), ("out", "out", "data")],
    },
    NodeTypeEntry {
        type_name: "quantized",
        crate_name: "vil_quantized",
        category: "inference",
        color: "#AD1457",
        description: "Local quantized model inference (Q4/Q8/F16)",
        default_ports: &[("in", "in", "trigger"), ("out", "out", "data")],
    },
    NodeTypeEntry {
        type_name: "tensor_pool",
        crate_name: "vil_tensor_shm",
        category: "inference",
        color: "#AD1457",
        description: "Zero-copy tensor serving via SHM ring buffer",
        default_ports: &[("in", "in", "data"), ("out", "out", "data")],
    },
    // ═══════════════════════════════════════════════════════════════════════
    // Data: DB / Cache / MQ (Batch E — from vil_db_* crates)
    // ═══════════════════════════════════════════════════════════════════════
    NodeTypeEntry {
        type_name: "db_query",
        crate_name: "vil_db_sqlx",
        category: "database",
        color: "#1565C0",
        description: "SQL query (SELECT) against a named connection pool",
        default_ports: &[("in", "in", "trigger"), ("out", "out", "data")],
    },
    NodeTypeEntry {
        type_name: "db_insert",
        crate_name: "vil_db_sqlx",
        category: "database",
        color: "#1565C0",
        description: "SQL insert into a named connection pool",
        default_ports: &[("in", "in", "data"), ("out", "out", "data")],
    },
    NodeTypeEntry {
        type_name: "db_update",
        crate_name: "vil_db_sqlx",
        category: "database",
        color: "#1565C0",
        description: "SQL update against a named connection pool",
        default_ports: &[("in", "in", "data"), ("out", "out", "data")],
    },
    NodeTypeEntry {
        type_name: "db_delete",
        crate_name: "vil_db_sqlx",
        category: "database",
        color: "#1565C0",
        description: "SQL delete against a named connection pool",
        default_ports: &[("in", "in", "data"), ("out", "out", "data")],
    },
    NodeTypeEntry {
        type_name: "cache_get",
        crate_name: "vil_cache",
        category: "cache",
        color: "#E65100",
        description: "Cache read (Redis, SHM, or in-memory)",
        default_ports: &[("in", "in", "trigger"), ("out", "out", "data")],
    },
    NodeTypeEntry {
        type_name: "cache_set",
        crate_name: "vil_cache",
        category: "cache",
        color: "#E65100",
        description: "Cache write with TTL",
        default_ports: &[("in", "in", "data"), ("out", "out", "data")],
    },
    NodeTypeEntry {
        type_name: "cache_del",
        crate_name: "vil_cache",
        category: "cache",
        color: "#E65100",
        description: "Cache invalidation",
        default_ports: &[("in", "in", "trigger"), ("out", "out", "data")],
    },
    NodeTypeEntry {
        type_name: "mq_publish",
        crate_name: "vil_mq_nats",
        category: "mq",
        color: "#E65100",
        description: "Publish message to NATS/Kafka topic",
        default_ports: &[("in", "in", "data")],
    },
    NodeTypeEntry {
        type_name: "mq_subscribe",
        crate_name: "vil_mq_nats",
        category: "mq",
        color: "#E65100",
        description: "Subscribe to NATS/Kafka topic",
        default_ports: &[("out", "out", "trigger")],
    },
    NodeTypeEntry {
        type_name: "vector_search",
        crate_name: "vil_vectordb",
        category: "data",
        color: "#1565C0",
        description: "HNSW vector search (native single-binary index)",
        default_ports: &[("in", "in", "trigger"), ("out", "out", "data")],
    },
    NodeTypeEntry {
        type_name: "vector_ingest",
        crate_name: "vil_vectordb",
        category: "data",
        color: "#1565C0",
        description: "Vector index ingestion (embed + store)",
        default_ports: &[("in", "in", "data"), ("out", "out", "data")],
    },
    // ═══════════════════════════════════════════════════════════════════════
    // Connectors: NATS (pub/sub, JetStream, KV)
    // ═══════════════════════════════════════════════════════════════════════
    NodeTypeEntry {
        type_name: "nats_publish",
        crate_name: "vil_mq_nats",
        category: "nats",
        color: "#27AE60",
        description: "Publish message to NATS subject",
        default_ports: &[("in", "in", "data")],
    },
    NodeTypeEntry {
        type_name: "nats_subscribe",
        crate_name: "vil_mq_nats",
        category: "nats",
        color: "#27AE60",
        description: "Subscribe to NATS subject (wildcard: *, >)",
        default_ports: &[("out", "out", "trigger")],
    },
    NodeTypeEntry {
        type_name: "nats_request",
        crate_name: "vil_mq_nats",
        category: "nats",
        color: "#27AE60",
        description: "NATS request/reply pattern",
        default_ports: &[("in", "in", "trigger"), ("out", "out", "data")],
    },
    NodeTypeEntry {
        type_name: "nats_jetstream",
        crate_name: "vil_mq_nats",
        category: "nats",
        color: "#27AE60",
        description: "NATS JetStream persistent publish (durable, replay)",
        default_ports: &[("in", "in", "data"), ("out", "out", "data")],
    },
    NodeTypeEntry {
        type_name: "nats_kv_get",
        crate_name: "vil_mq_nats",
        category: "nats",
        color: "#27AE60",
        description: "NATS KV store read (distributed key-value)",
        default_ports: &[("in", "in", "trigger"), ("out", "out", "data")],
    },
    NodeTypeEntry {
        type_name: "nats_kv_put",
        crate_name: "vil_mq_nats",
        category: "nats",
        color: "#27AE60",
        description: "NATS KV store write",
        default_ports: &[("in", "in", "data"), ("out", "out", "data")],
    },
    // ═══════════════════════════════════════════════════════════════════════
    // Connectors: Kafka
    // ═══════════════════════════════════════════════════════════════════════
    NodeTypeEntry {
        type_name: "kafka_produce",
        crate_name: "vil_mq_kafka",
        category: "kafka",
        color: "#231F20",
        description: "Kafka producer (keyed partitioning, SASL auth)",
        default_ports: &[("in", "in", "data")],
    },
    NodeTypeEntry {
        type_name: "kafka_consume",
        crate_name: "vil_mq_kafka",
        category: "kafka",
        color: "#231F20",
        description: "Kafka consumer (consumer groups, offset management)",
        default_ports: &[("out", "out", "trigger")],
    },
    // ═══════════════════════════════════════════════════════════════════════
    // Connectors: MQTT (IoT)
    // ═══════════════════════════════════════════════════════════════════════
    NodeTypeEntry {
        type_name: "mqtt_publish",
        crate_name: "vil_mq_mqtt",
        category: "mqtt",
        color: "#660099",
        description: "MQTT publish (QoS 0/1/2, TLS, keepalive)",
        default_ports: &[("in", "in", "data")],
    },
    NodeTypeEntry {
        type_name: "mqtt_subscribe",
        crate_name: "vil_mq_mqtt",
        category: "mqtt",
        color: "#660099",
        description: "MQTT subscribe with topic wildcards (+, #)",
        default_ports: &[("out", "out", "trigger")],
    },
    // ═══════════════════════════════════════════════════════════════════════
    // Connectors: gRPC
    // ═══════════════════════════════════════════════════════════════════════
    NodeTypeEntry {
        type_name: "grpc_server",
        crate_name: "vil_grpc",
        category: "grpc",
        color: "#244C5A",
        description: "gRPC server endpoint (tonic, health check, reflection)",
        default_ports: &[("in", "in", "trigger"), ("out", "out", "data")],
    },
    NodeTypeEntry {
        type_name: "grpc_client",
        crate_name: "vil_grpc",
        category: "grpc",
        color: "#244C5A",
        description: "gRPC client call to upstream service",
        default_ports: &[
            ("in", "in", "trigger"),
            ("out", "out", "data"),
            ("ctrl", "out", "control"),
        ],
    },
    // ═══════════════════════════════════════════════════════════════════════
    // Connectors: WebSocket + SSE (real-time)
    // ═══════════════════════════════════════════════════════════════════════
    NodeTypeEntry {
        type_name: "ws_server",
        crate_name: "vil_server_core",
        category: "websocket",
        color: "#F57C00",
        description: "WebSocket server endpoint (broadcast hub, topic routing)",
        default_ports: &[
            ("in", "in", "data"),
            ("out", "out", "trigger"),
            ("broadcast", "out", "data"),
        ],
    },
    NodeTypeEntry {
        type_name: "ws_client",
        crate_name: "vil_server_core",
        category: "websocket",
        color: "#F57C00",
        description: "WebSocket client connection to upstream",
        default_ports: &[("in", "in", "trigger"), ("out", "out", "data")],
    },
    NodeTypeEntry {
        type_name: "sse_hub",
        crate_name: "vil_server_core",
        category: "sse",
        color: "#F57C00",
        description: "SSE fan-out hub (broadcast to N connected clients)",
        default_ports: &[("in", "in", "data"), ("stream", "out", "data")],
    },
    // ═══════════════════════════════════════════════════════════════════════
    // Connectors: Sidecar (external process via SHM + UDS)
    // ═══════════════════════════════════════════════════════════════════════
    NodeTypeEntry {
        type_name: "sidecar",
        crate_name: "vil_sidecar",
        category: "sidecar",
        color: "#795548",
        description: "External process via SHM IPC (Python, Go, Java, etc.)",
        default_ports: &[
            ("in", "in", "trigger"),
            ("out", "out", "data"),
            ("ctrl", "out", "control"),
        ],
    },
    NodeTypeEntry {
        type_name: "sidecar_pool",
        crate_name: "vil_sidecar",
        category: "sidecar",
        color: "#795548",
        description: "Connection pool to sidecar with failover + circuit breaker",
        default_ports: &[("in", "in", "trigger"), ("out", "out", "data")],
    },
];

/// Lookup a node type by name.
pub fn find_node_type(type_name: &str) -> Option<&'static NodeTypeEntry> {
    NODE_TYPES.iter().find(|e| e.type_name == type_name)
}

/// Get all unique categories.
pub fn categories() -> Vec<&'static str> {
    let mut cats: Vec<&str> = NODE_TYPES.iter().map(|e| e.category).collect();
    cats.sort();
    cats.dedup();
    cats
}

/// List all node types, optionally filtered by category.
pub fn list_node_types(category_filter: Option<&str>) -> Vec<&'static NodeTypeEntry> {
    NODE_TYPES
        .iter()
        .filter(|e| category_filter.map_or(true, |c| e.category == c))
        .collect()
}
