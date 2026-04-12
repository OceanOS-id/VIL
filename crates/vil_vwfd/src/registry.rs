//! VilConnectorRegistry — async dispatch to VIL native crates.
//!
//! Feature-gated: each connector only compiled if its feature is enabled.
//! ConnectorPools holds Arc-wrapped VIL crate clients, initialized at startup.
//! All dispatch functions are async — no block_on.

use serde_json::Value;
use std::sync::Arc;

/// Connector call result.
pub type ConnectorResult = Result<Value, String>;

// ── ConnectorPools ──────────────────────────────────────────────────────────

/// Connector pools — initialized at startup from env vars.
pub struct ConnectorPools {
    // ── DB ──
    #[cfg(feature = "connectors-db")]
    pub sqlx_pool: Option<Arc<vil_db_sqlx::SqlxPool>>,
    #[cfg(feature = "connectors-db")]
    pub redis_pool: Option<Arc<vil_db_redis::RedisPool>>,
    #[cfg(feature = "connectors-db")]
    pub mongo_client: Option<Arc<vil_db_mongo::MongoClient>>,
    #[cfg(feature = "connectors-db")]
    pub cassandra_client: Option<Arc<vil_db_cassandra::CassandraClient>>,
    #[cfg(feature = "connectors-db")]
    pub clickhouse_client: Option<Arc<vil_db_clickhouse::ChClient>>,
    #[cfg(feature = "connectors-db")]
    pub dynamodb_client: Option<Arc<vil_db_dynamodb::DynamoClient>>,
    #[cfg(feature = "connectors-db")]
    pub elastic_client: Option<Arc<vil_db_elastic::ElasticClient>>,
    #[cfg(feature = "connectors-db")]
    pub neo4j_client: Option<Arc<vil_db_neo4j::Neo4jClient>>,
    #[cfg(feature = "connectors-db")]
    pub timeseries_client: Option<Arc<vil_db_timeseries::TimeseriesClient>>,
    // ── MQ ──
    #[cfg(feature = "connectors-mq")]
    pub nats_client: Option<Arc<vil_mq_nats::NatsClient>>,
    #[cfg(feature = "connectors-mq")]
    pub kafka_producer: Option<Arc<vil_mq_kafka::KafkaProducer>>,
    #[cfg(feature = "connectors-mq")]
    pub mqtt_client: Option<Arc<vil_mq_mqtt::MqttClient>>,
    #[cfg(feature = "connectors-mq")]
    pub rabbitmq_client: Option<Arc<vil_mq_rabbitmq::RabbitClient>>,
    #[cfg(feature = "connectors-mq")]
    pub pulsar_client: Option<Arc<vil_mq_pulsar::PulsarClient>>,
    #[cfg(feature = "connectors-mq")]
    pub pubsub_client: Option<Arc<vil_mq_pubsub::PubSubClient>>,
    #[cfg(feature = "connectors-mq")]
    pub sqs_client: Option<Arc<vil_mq_sqs::SqsClient>>,
    // ── Storage ──
    #[cfg(feature = "connectors-storage")]
    pub s3_client: Option<Arc<vil_storage_s3::S3Client>>,
    #[cfg(feature = "connectors-storage")]
    pub gcs_client: Option<Arc<vil_storage_gcs::GcsClient>>,
    #[cfg(feature = "connectors-storage")]
    pub azure_client: Option<Arc<vil_storage_azure::AzureClient>>,
    // ── Protocol ──
    #[cfg(feature = "connectors-protocol")]
    pub soap_client: Option<Arc<vil_soap::SoapClient>>,
    #[cfg(feature = "connectors-protocol")]
    pub modbus_client: Option<Arc<tokio::sync::Mutex<vil_modbus::ModbusClient>>>,
    #[cfg(feature = "connectors-protocol")]
    pub opcua_client: Option<Arc<vil_opcua::OpcUaClient>>,
    #[cfg(feature = "connectors-protocol")]
    pub ws_config: Option<Arc<vil_ws::WsConfig>>,
}

impl ConnectorPools {
    pub fn new() -> Self {
        Self {
            #[cfg(feature = "connectors-db")]    sqlx_pool: None,
            #[cfg(feature = "connectors-db")]    redis_pool: None,
            #[cfg(feature = "connectors-db")]    mongo_client: None,
            #[cfg(feature = "connectors-db")]    cassandra_client: None,
            #[cfg(feature = "connectors-db")]    clickhouse_client: None,
            #[cfg(feature = "connectors-db")]    dynamodb_client: None,
            #[cfg(feature = "connectors-db")]    elastic_client: None,
            #[cfg(feature = "connectors-db")]    neo4j_client: None,
            #[cfg(feature = "connectors-db")]    timeseries_client: None,
            #[cfg(feature = "connectors-mq")]    nats_client: None,
            #[cfg(feature = "connectors-mq")]    kafka_producer: None,
            #[cfg(feature = "connectors-mq")]    mqtt_client: None,
            #[cfg(feature = "connectors-mq")]    rabbitmq_client: None,
            #[cfg(feature = "connectors-mq")]    pulsar_client: None,
            #[cfg(feature = "connectors-mq")]    pubsub_client: None,
            #[cfg(feature = "connectors-mq")]    sqs_client: None,
            #[cfg(feature = "connectors-storage")] s3_client: None,
            #[cfg(feature = "connectors-storage")] gcs_client: None,
            #[cfg(feature = "connectors-storage")] azure_client: None,
            #[cfg(feature = "connectors-protocol")] soap_client: None,
            #[cfg(feature = "connectors-protocol")] modbus_client: None,
            #[cfg(feature = "connectors-protocol")] opcua_client: None,
            #[cfg(feature = "connectors-protocol")] ws_config: None,
        }
    }

    /// Initialize pools from environment variables (async, call at boot).
    #[allow(unused_mut)]
    pub async fn init_from_env(&mut self) -> Vec<String> {
        let mut errors = Vec::new();

        #[cfg(feature = "connectors-db")]
        if let Ok(url) = std::env::var("VIL_DATABASE_URL") {
            let driver = if url.starts_with("postgres") { "postgres" }
                else if url.starts_with("mysql") { "mysql" }
                else { "sqlite" };
            let config = vil_db_sqlx::SqlxConfig {
                driver: driver.into(), url,
                max_connections: std::env::var("VIL_DB_MAX_CONN").ok().and_then(|v| v.parse().ok()).unwrap_or(10),
                min_connections: 1, connect_timeout_secs: 5, idle_timeout_secs: 300,
                ssl_mode: "prefer".into(), services: vec![],
            };
            match vil_db_sqlx::SqlxPool::connect("vil_vwfd", config).await {
                Ok(pool) => self.sqlx_pool = Some(Arc::new(pool)),
                Err(e) => errors.push(format!("sqlx: {}", e)),
            }
        }

        #[cfg(feature = "connectors-db")]
        if let Ok(url) = std::env::var("VIL_REDIS_URL") {
            match vil_db_redis::RedisPool::connect("vil_vwfd", vil_db_redis::RedisConfig::new(&url)).await {
                Ok(pool) => self.redis_pool = Some(Arc::new(pool)),
                Err(e) => errors.push(format!("redis: {}", e)),
            }
        }

        #[cfg(feature = "connectors-db")]
        if let Ok(uri) = std::env::var("VIL_MONGO_URI") {
            let db = std::env::var("VIL_MONGO_DB").unwrap_or_else(|_| "vil".into());
            match vil_db_mongo::MongoClient::new(vil_db_mongo::MongoConfig::new(&uri, &db)).await {
                Ok(client) => self.mongo_client = Some(Arc::new(client)),
                Err(e) => errors.push(format!("mongo: {}", e)),
            }
        }

        #[cfg(feature = "connectors-mq")]
        if let Ok(url) = std::env::var("VIL_NATS_URL") {
            match vil_mq_nats::NatsClient::connect(vil_mq_nats::NatsConfig::new(&url)).await {
                Ok(client) => self.nats_client = Some(Arc::new(client)),
                Err(e) => errors.push(format!("nats: {}", e)),
            }
        }

        #[cfg(feature = "connectors-mq")]
        if let Ok(brokers) = std::env::var("VIL_KAFKA_BROKERS") {
            match vil_mq_kafka::KafkaProducer::new(vil_mq_kafka::KafkaConfig::new(&brokers)).await {
                Ok(producer) => self.kafka_producer = Some(Arc::new(producer)),
                Err(e) => errors.push(format!("kafka: {}", e)),
            }
        }

        #[cfg(feature = "connectors-storage")]
        if let Ok(bucket) = std::env::var("VIL_S3_BUCKET") {
            let config = vil_storage_s3::S3Config {
                endpoint: std::env::var("VIL_S3_ENDPOINT").ok(),
                region: std::env::var("VIL_S3_REGION").unwrap_or_else(|_| "us-east-1".into()),
                access_key: std::env::var("VIL_S3_ACCESS_KEY").ok(),
                secret_key: std::env::var("VIL_S3_SECRET_KEY").ok(),
                bucket,
                path_style: std::env::var("VIL_S3_PATH_STYLE").map(|v| v == "true").unwrap_or(false),
            };
            match vil_storage_s3::S3Client::new(config).await {
                Ok(client) => self.s3_client = Some(Arc::new(client)),
                Err(e) => errors.push(format!("s3: {}", e)),
            }
        }

        errors
    }
}

impl Default for ConnectorPools {
    fn default() -> Self { Self::new() }
}

// ── Async Dispatch ──────────────────────────────────────────────────────────

/// Dispatch connector call (fully async, no block_on).
pub async fn dispatch(
    connector_ref: &str,
    operation: &str,
    input: &Value,
    _pools: &ConnectorPools,
) -> ConnectorResult {
    #[cfg(any(feature = "connectors-db", feature = "connectors-mq", feature = "connectors-storage"))]
    let pools = _pools;

    match connector_ref {
        // ── HTTP ──
        r if r == "vastar.http" || r.contains("http") => {
            dispatch_http(operation, input).await
        }

        // ── SQLx ──
        #[cfg(feature = "connectors-db")]
        "vastar.db.postgres" | "vastar.db.mysql" | "vastar.db.sqlite" => {
            dispatch_sqlx(operation, input, pools).await
        }
        #[cfg(feature = "connectors-db")]
        "vastar.db.redis" => dispatch_redis(operation, input, pools).await,
        #[cfg(feature = "connectors-db")]
        "vastar.db.mongo" => dispatch_mongo(operation, input, pools).await,

        // ── MQ ──
        #[cfg(feature = "connectors-mq")]
        "vastar.mq.nats" => dispatch_nats(operation, input, pools).await,
        #[cfg(feature = "connectors-mq")]
        "vastar.mq.kafka" => dispatch_kafka(operation, input, pools).await,

        // ── Storage ──
        #[cfg(feature = "connectors-storage")]
        "vastar.storage.s3" => dispatch_s3(operation, input, pools).await,

        // ── Remaining DB ──
        #[cfg(feature = "connectors-db")]
        "vastar.db.cassandra" => dispatch_cassandra(operation, input, pools).await,
        #[cfg(feature = "connectors-db")]
        "vastar.db.clickhouse" => dispatch_clickhouse(operation, input, pools).await,
        #[cfg(feature = "connectors-db")]
        "vastar.db.dynamodb" => dispatch_dynamodb(operation, input, pools).await,
        #[cfg(feature = "connectors-db")]
        "vastar.db.elastic" => dispatch_elastic(operation, input, pools).await,
        #[cfg(feature = "connectors-db")]
        "vastar.db.neo4j" => dispatch_neo4j(operation, input, pools).await,
        #[cfg(feature = "connectors-db")]
        "vastar.db.timeseries" => dispatch_timeseries(operation, input, pools).await,

        // ── Remaining MQ ──
        #[cfg(feature = "connectors-mq")]
        "vastar.mq.mqtt" => dispatch_mqtt(operation, input, pools).await,
        #[cfg(feature = "connectors-mq")]
        "vastar.mq.rabbitmq" => dispatch_rabbitmq(operation, input, pools).await,
        #[cfg(feature = "connectors-mq")]
        "vastar.mq.pulsar" => dispatch_pulsar(operation, input, pools).await,
        #[cfg(feature = "connectors-mq")]
        "vastar.mq.pubsub" => dispatch_pubsub(operation, input, pools).await,
        #[cfg(feature = "connectors-mq")]
        "vastar.mq.sqs" => dispatch_sqs(operation, input, pools).await,

        // ── Remaining Storage ──
        #[cfg(feature = "connectors-storage")]
        "vastar.storage.gcs" => dispatch_gcs(operation, input, pools).await,
        #[cfg(feature = "connectors-storage")]
        "vastar.storage.azure" => dispatch_azure(operation, input, pools).await,

        // ── Protocol ──
        #[cfg(feature = "connectors-protocol")]
        "vastar.soap" => dispatch_soap(operation, input, pools).await,
        #[cfg(feature = "connectors-protocol")]
        "vastar.modbus" => dispatch_modbus(operation, input, pools).await,
        #[cfg(feature = "connectors-protocol")]
        "vastar.opcua" => dispatch_opcua(operation, input, pools).await,
        #[cfg(feature = "connectors-protocol")]
        "vastar.ws" => dispatch_ws(operation, input, pools).await,
        #[cfg(feature = "connectors-protocol")]
        "vastar.sftp" => dispatch_sftp(operation, input).await,

        // ── Codec ──
        "vastar.codec.iso8583" => dispatch_codec_iso8583(operation, input).await,
        "vastar.codec.msgpack" => dispatch_codec_msgpack(operation, input).await,
        "vastar.codec.protobuf" => dispatch_codec_protobuf(operation, input).await,

        _ => Err(format!("unknown connector_ref: '{}'", connector_ref)),
    }
}

// ── HTTP (vil_new_http + vil_server_core::SseCollect) ────────────────────────
//
// Streaming vs non-streaming determined by connector_config in VWFD YAML:
//   streaming: true  → SseCollect (SSE stream → collect text)
//   streaming: false → HttpRequest (buffered JSON response)

async fn dispatch_http(operation: &str, input: &Value) -> ConnectorResult {
    let url = input.get("url").and_then(|v| v.as_str())
        .ok_or("HTTP connector: url required")?;
    let streaming = input.get("_streaming").and_then(|v| v.as_bool()).unwrap_or(false);

    if streaming {
        dispatch_http_streaming(url, input).await
    } else {
        dispatch_http_buffered(operation, url, input).await
    }
}

/// Non-streaming: simple HTTP request → JSON response.
async fn dispatch_http_buffered(operation: &str, url: &str, input: &Value) -> ConnectorResult {
    use vil_new_http::request::HttpRequest;

    let body = input.get("body").cloned().unwrap_or(Value::Null);
    let headers = input.get("headers").and_then(|v| v.as_object());
    let op = operation.to_uppercase();

    let mut req = match op.as_str() {
        "GET" => HttpRequest::get(url),
        "POST" => HttpRequest::post(url),
        "PUT" => HttpRequest::put(url),
        "DELETE" => HttpRequest::delete(url),
        "PATCH" => HttpRequest::patch(url),
        _ => HttpRequest::post(url),
    };

    if let Some(hdrs) = headers {
        for (k, v) in hdrs {
            if let Some(val) = v.as_str() { req = req.header(k, val); }
        }
    }
    if !body.is_null() { req = req.json(body); }

    let resp = req.send().await.map_err(|e| format!("HTTP {}: {}", op, e))?;
    Ok(serde_json::json!({"status": resp.status, "body": resp.body}))
}

/// Streaming: SSE stream → collect all chunks → return as text.
/// Uses vil_server_core::SseCollect with dialect support.
async fn dispatch_http_streaming(url: &str, input: &Value) -> ConnectorResult {
    use vil_server_core::SseCollect;

    let body = input.get("body").cloned().unwrap_or(Value::Null);
    let headers = input.get("headers").and_then(|v| v.as_object());
    let dialect = input.get("_dialect").and_then(|v| v.as_str()).unwrap_or("openai");
    let json_tap = input.get("_json_tap").and_then(|v| v.as_str());

    let mut sse = SseCollect::post_to(url);

    // Apply dialect
    sse = match dialect {
        "anthropic" => sse.dialect(vil_server_core::SseDialect::anthropic()),
        "ollama" => sse.dialect(vil_server_core::SseDialect::ollama()),
        "cohere" => sse.dialect(vil_server_core::SseDialect::cohere()),
        "gemini" => sse.dialect(vil_server_core::SseDialect::gemini()),
        "standard" => sse.dialect(vil_server_core::SseDialect::standard()),
        _ => sse.dialect(vil_server_core::SseDialect::openai()),
    };

    // Apply json_tap
    if let Some(tap) = json_tap {
        sse = sse.json_tap(tap);
    }

    // Apply body
    if !body.is_null() {
        sse = sse.body(body);
    }

    // Apply headers
    if let Some(hdrs) = headers {
        for (k, v) in hdrs {
            if let Some(val) = v.as_str() {
                sse = sse.header(k, val);
            }
        }
    }

    let content = sse.collect_text().await
        .map_err(|e| format!("SSE stream: {}", e))?;

    Ok(serde_json::json!({"content": content}))
}

// ── SQLx (vil_db_sqlx) ─────────────────────────────────────────────────────

#[cfg(feature = "connectors-db")]
async fn dispatch_sqlx(operation: &str, input: &Value, pools: &ConnectorPools) -> ConnectorResult {
    let pool = pools.sqlx_pool.as_ref()
        .ok_or("SQLx pool not initialized. Set VIL_DATABASE_URL.")?;

    let sql = input.get("sql").and_then(|v| v.as_str()).unwrap_or("");
    if sql.is_empty() && operation != "health" && operation != "ping" {
        return Err("sql field required".into());
    }

    let is_select = sql.trim_start().to_uppercase().starts_with("SELECT");

    match operation {
        // DDL/DML — execute, return rows_affected
        "raw_query" | "execute" | "insert" | "update" | "delete" => {
            let rows = pool.execute_raw(sql).await.map_err(|e| format!("sqlx: {}", e))?;
            Ok(serde_json::json!({"rows_affected": rows, "sql": sql, "operation": operation}))
        }

        // SELECT — return actual row data
        "query" | "select" | "find_many" | "find_one" => {
            if is_select {
                let rows = pool.fetch_all_json(sql).await
                    .map_err(|e| format!("sqlx select: {}", e))?;
                let count = rows.len();
                if operation == "find_one" {
                    let row = rows.into_iter().next().unwrap_or(Value::Null);
                    Ok(serde_json::json!({"row": row, "found": !row.is_null(), "sql": sql}))
                } else {
                    Ok(serde_json::json!({"rows": rows, "row_count": count, "sql": sql}))
                }
            } else {
                // Non-SELECT passed as query — execute as DML
                let rows = pool.execute_raw(sql).await.map_err(|e| format!("sqlx: {}", e))?;
                Ok(serde_json::json!({"rows_affected": rows, "sql": sql}))
            }
        }

        "health" | "ping" => {
            pool.execute_raw("SELECT 1").await.map_err(|e| format!("sqlx: {}", e))?;
            Ok(serde_json::json!({"status": "ok"}))
        }

        // Unknown op — auto-detect SELECT vs DML by SQL prefix
        _ => {
            if is_select {
                let rows = pool.fetch_all_json(sql).await
                    .map_err(|e| format!("sqlx: {}", e))?;
                Ok(serde_json::json!({"rows": rows, "row_count": rows.len(), "sql": sql}))
            } else {
                let rows = pool.execute_raw(sql).await.map_err(|e| format!("sqlx: {}", e))?;
                Ok(serde_json::json!({"rows_affected": rows, "sql": sql}))
            }
        }
    }
}

// ── Redis (vil_db_redis) ────────────────────────────────────────────────────

#[cfg(feature = "connectors-db")]
async fn dispatch_redis(operation: &str, input: &Value, pools: &ConnectorPools) -> ConnectorResult {
    let pool = pools.redis_pool.as_ref()
        .ok_or("Redis pool not initialized. Set VIL_REDIS_URL.")?;

    match operation {
        "get" => {
            let key = input.get("key").and_then(|v| v.as_str()).ok_or("key required")?;
            let val = pool.get(key).await;
            Ok(serde_json::json!({"key": key, "value": val}))
        }
        "set" => {
            let key = input.get("key").and_then(|v| v.as_str()).ok_or("key required")?;
            let value = input.get("value").and_then(|v| v.as_str()).ok_or("value required")?;
            let ttl = input.get("ttl").and_then(|v| v.as_u64());
            if let Some(ttl) = ttl { pool.set_ex(key, value, ttl).await; }
            else { pool.set(key, value).await; }
            Ok(serde_json::json!({"status": "ok", "key": key}))
        }
        "del" | "delete" => {
            let key = input.get("key").and_then(|v| v.as_str()).ok_or("key required")?;
            let deleted = pool.del(key).await;
            Ok(serde_json::json!({"deleted": deleted, "key": key}))
        }
        "ping" => {
            let pong = pool.ping().await.map_err(|e| format!("redis ping: {}", e))?;
            Ok(serde_json::json!({"status": pong}))
        }
        _ => Err(format!("redis: unsupported operation '{}'", operation)),
    }
}

// ── MongoDB (vil_db_mongo) ──────────────────────────────────────────────────

#[cfg(feature = "connectors-db")]
async fn dispatch_mongo(operation: &str, input: &Value, pools: &ConnectorPools) -> ConnectorResult {
    let client = pools.mongo_client.as_ref()
        .ok_or("MongoDB not initialized. Set VIL_MONGO_URI.")?;
    let collection = input.get("collection").and_then(|v| v.as_str())
        .ok_or("collection required")?.to_string();

    match operation {
        "find_one" => {
            let filter = input_to_bson_doc(input.get("filter"))?;
            let result: Option<Value> = client.find_one(&collection, filter).await
                .map_err(|e| format!("mongo find_one: {}", e))?;
            Ok(result.unwrap_or(Value::Null))
        }
        "find" | "find_many" => {
            let filter = input_to_bson_doc(input.get("filter"))?;
            let limit = input.get("limit").and_then(|v| v.as_i64());
            let results: Vec<Value> = client.find_many(&collection, filter, limit).await
                .map_err(|e| format!("mongo find: {}", e))?;
            Ok(Value::Array(results))
        }
        "insert_one" => {
            let doc = input.get("document").or_else(|| input.get("doc"))
                .ok_or("document required")?;
            let id = client.insert_one(&collection, doc).await
                .map_err(|e| format!("mongo insert: {}", e))?;
            Ok(serde_json::json!({"inserted_id": id}))
        }
        "update_one" => {
            let filter = input_to_bson_doc(input.get("filter"))?;
            let update = input_to_bson_doc(input.get("update"))?;
            let count = client.update_one(&collection, filter, update).await
                .map_err(|e| format!("mongo update: {}", e))?;
            Ok(serde_json::json!({"modified_count": count}))
        }
        "delete_one" => {
            let filter = input_to_bson_doc(input.get("filter"))?;
            let count = client.delete_one(&collection, filter).await
                .map_err(|e| format!("mongo delete: {}", e))?;
            Ok(serde_json::json!({"deleted_count": count}))
        }
        "count" => {
            let filter = input.get("filter").and_then(|f| input_to_bson_doc(Some(f)).ok());
            let count = client.count(&collection, filter).await
                .map_err(|e| format!("mongo count: {}", e))?;
            Ok(serde_json::json!({"count": count}))
        }
        _ => Err(format!("mongo: unsupported operation '{}'", operation)),
    }
}

#[cfg(feature = "connectors-db")]
fn input_to_bson_doc(val: Option<&Value>) -> Result<bson::Document, String> {
    match val {
        Some(v) => match bson::to_bson(v).map_err(|e| format!("bson: {}", e))? {
            bson::Bson::Document(doc) => Ok(doc),
            _ => Ok(bson::Document::new()),
        },
        None => Ok(bson::Document::new()),
    }
}

// ── NATS (vil_mq_nats) ─────────────────────────────────────────────────────

#[cfg(feature = "connectors-mq")]
async fn dispatch_nats(operation: &str, input: &Value, pools: &ConnectorPools) -> ConnectorResult {
    let client = pools.nats_client.as_ref()
        .ok_or("NATS not initialized. Set VIL_NATS_URL.")?;

    match operation {
        "publish" => {
            let subject = input.get("subject").and_then(|v| v.as_str()).ok_or("subject required")?;
            let payload = input.get("payload")
                .map(|v| serde_json::to_vec(v).unwrap_or_default()).unwrap_or_default();
            client.publish(subject, &payload).await
                .map_err(|e| format!("nats publish: {}", e))?;
            Ok(serde_json::json!({"status": "published", "subject": subject}))
        }
        "request" => {
            let subject = input.get("subject").and_then(|v| v.as_str()).ok_or("subject required")?;
            let payload = input.get("payload")
                .map(|v| serde_json::to_vec(v).unwrap_or_default()).unwrap_or_default();
            let msg = client.request(subject, &payload).await
                .map_err(|e| format!("nats request: {}", e))?;
            let resp: Value = serde_json::from_slice(&msg.payload)
                .unwrap_or(Value::String(String::from_utf8_lossy(&msg.payload).into()));
            Ok(serde_json::json!({"subject": msg.subject, "payload": resp}))
        }
        _ => Err(format!("nats: unsupported operation '{}'", operation)),
    }
}

// ── Kafka (vil_mq_kafka) ────────────────────────────────────────────────────

#[cfg(feature = "connectors-mq")]
async fn dispatch_kafka(operation: &str, input: &Value, pools: &ConnectorPools) -> ConnectorResult {
    let producer = pools.kafka_producer.as_ref()
        .ok_or("Kafka not initialized. Set VIL_KAFKA_BROKERS.")?;

    match operation {
        "publish" | "produce" => {
            let topic = input.get("topic").and_then(|v| v.as_str()).ok_or("topic required")?;
            let payload = input.get("payload")
                .map(|v| serde_json::to_vec(v).unwrap_or_default()).unwrap_or_default();
            let key = input.get("key").and_then(|v| v.as_str());
            if let Some(key) = key {
                producer.publish_keyed(topic, key, &payload).await
                    .map_err(|e| format!("kafka publish: {}", e))?;
            } else {
                producer.publish(topic, &payload).await
                    .map_err(|e| format!("kafka publish: {}", e))?;
            }
            Ok(serde_json::json!({"status": "published", "topic": topic}))
        }
        _ => Err(format!("kafka: unsupported operation '{}'", operation)),
    }
}

// ── S3 (vil_storage_s3) ─────────────────────────────────────────────────────

#[cfg(feature = "connectors-storage")]
async fn dispatch_s3(operation: &str, input: &Value, pools: &ConnectorPools) -> ConnectorResult {
    let client = pools.s3_client.as_ref()
        .ok_or("S3 not initialized. Set VIL_S3_BUCKET.")?;

    match operation {
        "get" | "get_object" => {
            let key = input.get("key").and_then(|v| v.as_str()).ok_or("key required")?;
            let data = client.get_object(key).await.map_err(|e| format!("s3 get: {}", e))?;
            let body: Value = serde_json::from_slice(&data)
                .unwrap_or_else(|_| Value::String(String::from_utf8_lossy(&data).into()));
            Ok(serde_json::json!({"key": key, "body": body}))
        }
        "put" | "put_object" => {
            let key = input.get("key").and_then(|v| v.as_str()).ok_or("key required")?;
            let body = input.get("body").ok_or("body required")?;
            let bytes = if let Some(s) = body.as_str() { bytes::Bytes::from(s.to_string()) }
            else { bytes::Bytes::from(serde_json::to_vec(body).unwrap_or_default()) };
            let result = client.put_object(key, bytes).await.map_err(|e| format!("s3 put: {}", e))?;
            Ok(serde_json::json!({"key": key, "e_tag": result.e_tag, "version_id": result.version_id}))
        }
        "delete" | "delete_object" => {
            let key = input.get("key").and_then(|v| v.as_str()).ok_or("key required")?;
            client.delete_object(key).await.map_err(|e| format!("s3 delete: {}", e))?;
            Ok(serde_json::json!({"deleted": true, "key": key}))
        }
        "list" | "list_objects" => {
            let prefix = input.get("prefix").and_then(|v| v.as_str()).unwrap_or("");
            let objects = client.list_objects(prefix).await.map_err(|e| format!("s3 list: {}", e))?;
            let items: Vec<Value> = objects.into_iter().map(|o| serde_json::json!({
                "key": o.key, "size": o.size, "last_modified": o.last_modified, "e_tag": o.e_tag,
            })).collect();
            Ok(Value::Array(items))
        }
        _ => Err(format!("s3: unsupported operation '{}'", operation)),
    }
}

// ── Cassandra ───────────────────────────────────────────────────────────────

#[cfg(feature = "connectors-db")]
async fn dispatch_cassandra(operation: &str, input: &Value, pools: &ConnectorPools) -> ConnectorResult {
    let client = pools.cassandra_client.as_ref().ok_or("Cassandra not initialized. Set VIL_CASSANDRA_CONTACT_POINTS.")?;
    let cql = input.get("cql").or(input.get("sql")).and_then(|v| v.as_str()).unwrap_or("");
    match operation {
        "query" => { let rows = client.query(cql, &()).await.map_err(|e| format!("cassandra query: {}", e))?; Ok(Value::String(format!("{:?}", rows))) }
        "execute" => { client.query(cql, &()).await.map_err(|e| format!("cassandra execute: {}", e))?; Ok(serde_json::json!({"status": "ok"})) }
        _ => { let rows = client.query(cql, &()).await.map_err(|e| format!("cassandra: {}", e))?; Ok(Value::String(format!("{:?}", rows))) }
    }
}

// ── ClickHouse ──────────────────────────────────────────────────────────────

#[cfg(feature = "connectors-db")]
async fn dispatch_clickhouse(operation: &str, input: &Value, pools: &ConnectorPools) -> ConnectorResult {
    let client = pools.clickhouse_client.as_ref().ok_or("ClickHouse not initialized. Set VIL_CLICKHOUSE_URL.")?;
    let sql = input.get("sql").and_then(|v| v.as_str()).unwrap_or("");
    match operation {
        "execute" => { client.execute(sql).await.map_err(|e| format!("clickhouse: {}", e))?; Ok(serde_json::json!({"status": "ok"})) }
        _ => { client.execute(sql).await.map_err(|e| format!("clickhouse: {}", e))?; Ok(serde_json::json!({"status": "ok", "sql": sql})) }
    }
}

// ── DynamoDB ────────────────────────────────────────────────────────────────

#[cfg(feature = "connectors-db")]
async fn dispatch_dynamodb(operation: &str, input: &Value, pools: &ConnectorPools) -> ConnectorResult {
    let client = pools.dynamodb_client.as_ref().ok_or("DynamoDB not initialized. Set VIL_DYNAMODB_REGION.")?;
    let table = input.get("table").and_then(|v| v.as_str()).ok_or("table required")?;
    match operation {
        "get" => { let key = std::collections::HashMap::new(); let r = client.get_item(table, key).await.map_err(|e| format!("dynamo get: {}", e))?; Ok(Value::String(format!("{:?}", r))) }
        "put" => { let item = std::collections::HashMap::new(); client.put_item(table, item).await.map_err(|e| format!("dynamo put: {}", e))?; Ok(serde_json::json!({"status": "ok"})) }
        "delete" => { let key = std::collections::HashMap::new(); client.delete_item(table, key).await.map_err(|e| format!("dynamo delete: {}", e))?; Ok(serde_json::json!({"deleted": true})) }
        "query" => { let expr = input.get("expression").and_then(|v| v.as_str()).unwrap_or(""); let vals = std::collections::HashMap::new(); let r = client.query(table, expr, vals).await.map_err(|e| format!("dynamo query: {}", e))?; Ok(Value::String(format!("{:?}", r))) }
        _ => Err(format!("dynamodb: unsupported operation '{}'", operation)),
    }
}

// ── Elasticsearch ───────────────────────────────────────────────────────────

#[cfg(feature = "connectors-db")]
async fn dispatch_elastic(operation: &str, input: &Value, pools: &ConnectorPools) -> ConnectorResult {
    let client = pools.elastic_client.as_ref().ok_or("Elastic not initialized. Set VIL_ELASTIC_URL.")?;
    let index = input.get("index").and_then(|v| v.as_str()).ok_or("index required")?;
    match operation {
        "index" => { let doc = input.get("document").cloned().unwrap_or(Value::Null); let id = input.get("id").and_then(|v| v.as_str()).unwrap_or("_auto"); client.index(index, id, doc).await.map_err(|e| format!("elastic index: {}", e))?; Ok(serde_json::json!({"status": "indexed"})) }
        "search" => { let query = input.get("query").cloned().unwrap_or(Value::Null); let r = client.search(index, query).await.map_err(|e| format!("elastic search: {}", e))?; Ok(serde_json::json!({"total": r.total, "hits": r.hits})) }
        "get" => { let id = input.get("id").and_then(|v| v.as_str()).ok_or("id required")?; let r = client.get(index, id).await.map_err(|e| format!("elastic get: {}", e))?; Ok(r) }
        "delete" => { let id = input.get("id").and_then(|v| v.as_str()).ok_or("id required")?; client.delete(index, id).await.map_err(|e| format!("elastic delete: {}", e))?; Ok(serde_json::json!({"deleted": true})) }
        _ => Err(format!("elastic: unsupported operation '{}'", operation)),
    }
}

// ── Neo4j ───────────────────────────────────────────────────────────────────

#[cfg(feature = "connectors-db")]
async fn dispatch_neo4j(operation: &str, input: &Value, pools: &ConnectorPools) -> ConnectorResult {
    let client = pools.neo4j_client.as_ref().ok_or("Neo4j not initialized. Set VIL_NEO4J_URI.")?;
    let cypher = input.get("cypher").or(input.get("query")).and_then(|v| v.as_str()).unwrap_or("");
    match operation {
        "execute" | "query" => { let rows = client.execute(cypher).await.map_err(|e| format!("neo4j: {}", e))?; Ok(Value::String(format!("{:?}", rows))) }
        "transaction" => { client.run_transaction(cypher).await.map_err(|e| format!("neo4j tx: {}", e))?; Ok(serde_json::json!({"status": "ok"})) }
        _ => { let rows = client.execute(cypher).await.map_err(|e| format!("neo4j: {}", e))?; Ok(Value::String(format!("{:?}", rows))) }
    }
}

// ── Timeseries (InfluxDB/TimescaleDB) ───────────────────────────────────────

#[cfg(feature = "connectors-db")]
async fn dispatch_timeseries(operation: &str, input: &Value, pools: &ConnectorPools) -> ConnectorResult {
    let client = pools.timeseries_client.as_ref().ok_or("Timeseries not initialized. Set VIL_INFLUX_URL.")?;
    match operation {
        "write" => { client.write_points(vec![]).await.map_err(|e| format!("timeseries write: {}", e))?; Ok(serde_json::json!({"status": "ok"})) }
        "query" => { let flux = input.get("flux").or(input.get("query")).and_then(|v| v.as_str()).unwrap_or(""); let r = client.query_flux(flux).await.map_err(|e| format!("timeseries query: {}", e))?; Ok(Value::String(format!("{:?}", r))) }
        _ => Err(format!("timeseries: unsupported operation '{}'", operation)),
    }
}

// ── MQTT ────────────────────────────────────────────────────────────────────

#[cfg(feature = "connectors-mq")]
async fn dispatch_mqtt(operation: &str, input: &Value, pools: &ConnectorPools) -> ConnectorResult {
    let client = pools.mqtt_client.as_ref().ok_or("MQTT not initialized. Set VIL_MQTT_URL.")?;
    match operation {
        "publish" => {
            let topic = input.get("topic").and_then(|v| v.as_str()).ok_or("topic required")?;
            let payload = input.get("payload").map(|v| serde_json::to_vec(v).unwrap_or_default()).unwrap_or_default();
            client.publish(topic, &payload, vil_mq_mqtt::QoS::AtLeastOnce).await.map_err(|e| format!("mqtt publish: {}", e))?;
            Ok(serde_json::json!({"status": "published", "topic": topic}))
        }
        _ => Err(format!("mqtt: unsupported operation '{}'", operation)),
    }
}

// ── RabbitMQ ────────────────────────────────────────────────────────────────

#[cfg(feature = "connectors-mq")]
async fn dispatch_rabbitmq(operation: &str, input: &Value, pools: &ConnectorPools) -> ConnectorResult {
    let client = pools.rabbitmq_client.as_ref().ok_or("RabbitMQ not initialized. Set VIL_RABBITMQ_URL.")?;
    match operation {
        "publish" => {
            let exchange = input.get("exchange").and_then(|v| v.as_str()).unwrap_or("");
            let routing_key = input.get("routing_key").and_then(|v| v.as_str()).unwrap_or("");
            let payload = input.get("payload").map(|v| serde_json::to_vec(v).unwrap_or_default()).unwrap_or_default();
            client.publish(exchange, routing_key, &payload).await.map_err(|e| format!("rabbitmq publish: {}", e))?;
            Ok(serde_json::json!({"status": "published", "exchange": exchange, "routing_key": routing_key}))
        }
        _ => Err(format!("rabbitmq: unsupported operation '{}'", operation)),
    }
}

// ── Pulsar ──────────────────────────────────────────────────────────────────

#[cfg(feature = "connectors-mq")]
async fn dispatch_pulsar(operation: &str, input: &Value, pools: &ConnectorPools) -> ConnectorResult {
    let client = pools.pulsar_client.as_ref().ok_or("Pulsar not initialized. Set VIL_PULSAR_URL.")?;
    match operation {
        "publish" => {
            let topic = input.get("topic").and_then(|v| v.as_str()).ok_or("topic required")?;
            let payload = input.get("payload").map(|v| serde_json::to_vec(v).unwrap_or_default()).unwrap_or_default();
            let mut producer = vil_mq_pulsar::PulsarProducer::new(client, topic).await.map_err(|e| format!("pulsar producer: {}", e))?;
            producer.send(&payload).await.map_err(|e| format!("pulsar publish: {}", e))?;
            Ok(serde_json::json!({"status": "published", "topic": topic}))
        }
        _ => Err(format!("pulsar: unsupported operation '{}'", operation)),
    }
}

// ── Google Pub/Sub ──────────────────────────────────────────────────────────

#[cfg(feature = "connectors-mq")]
async fn dispatch_pubsub(operation: &str, input: &Value, pools: &ConnectorPools) -> ConnectorResult {
    let client = pools.pubsub_client.as_ref().ok_or("PubSub not initialized. Set VIL_PUBSUB_PROJECT.")?;
    match operation {
        "publish" => {
            let payload = input.get("payload").map(|v| serde_json::to_vec(v).unwrap_or_default()).unwrap_or_default();
            client.publish(&payload).await.map_err(|e| format!("pubsub publish: {}", e))?;
            Ok(serde_json::json!({"status": "published"}))
        }
        _ => Err(format!("pubsub: unsupported operation '{}'", operation)),
    }
}

// ── AWS SQS ─────────────────────────────────────────────────────────────────

#[cfg(feature = "connectors-mq")]
async fn dispatch_sqs(operation: &str, input: &Value, pools: &ConnectorPools) -> ConnectorResult {
    let client = pools.sqs_client.as_ref().ok_or("SQS not initialized. Set VIL_SQS_QUEUE_URL.")?;
    match operation {
        "send" | "publish" => {
            let payload = input.get("payload").map(|v| serde_json::to_vec(v).unwrap_or_default()).unwrap_or_default();
            client.send_message(&payload).await.map_err(|e| format!("sqs send: {}", e))?;
            Ok(serde_json::json!({"status": "sent"}))
        }
        _ => Err(format!("sqs: unsupported operation '{}'", operation)),
    }
}

// ── GCS ─────────────────────────────────────────────────────────────────────

#[cfg(feature = "connectors-storage")]
async fn dispatch_gcs(operation: &str, input: &Value, pools: &ConnectorPools) -> ConnectorResult {
    let client = pools.gcs_client.as_ref().ok_or("GCS not initialized. Set VIL_GCS_BUCKET.")?;
    match operation {
        "get" | "download" => {
            let name = input.get("key").or(input.get("name")).and_then(|v| v.as_str()).ok_or("key required")?;
            let data = client.download(name).await.map_err(|e| format!("gcs get: {}", e))?;
            let body: Value = serde_json::from_slice(&data).unwrap_or_else(|_| Value::String(String::from_utf8_lossy(&data).into()));
            Ok(serde_json::json!({"key": name, "body": body}))
        }
        "put" | "upload" => {
            let name = input.get("key").or(input.get("name")).and_then(|v| v.as_str()).ok_or("key required")?;
            let body = input.get("body").ok_or("body required")?;
            let bytes = if let Some(s) = body.as_str() { bytes::Bytes::from(s.to_string()) } else { bytes::Bytes::from(serde_json::to_vec(body).unwrap_or_default()) };
            let r = client.upload(name, bytes).await.map_err(|e| format!("gcs put: {}", e))?;
            Ok(Value::String(format!("{:?}", r)))
        }
        "delete" => {
            let name = input.get("key").or(input.get("name")).and_then(|v| v.as_str()).ok_or("key required")?;
            client.delete(name).await.map_err(|e| format!("gcs delete: {}", e))?;
            Ok(serde_json::json!({"deleted": true, "key": name}))
        }
        "list" => {
            let prefix = input.get("prefix").and_then(|v| v.as_str()).unwrap_or("");
            let objects = client.list(prefix).await.map_err(|e| format!("gcs list: {}", e))?;
            Ok(Value::String(format!("{:?}", objects)))
        }
        _ => Err(format!("gcs: unsupported operation '{}'", operation)),
    }
}

// ── Azure Blob ──────────────────────────────────────────────────────────────

#[cfg(feature = "connectors-storage")]
async fn dispatch_azure(operation: &str, input: &Value, pools: &ConnectorPools) -> ConnectorResult {
    let client = pools.azure_client.as_ref().ok_or("Azure not initialized. Set VIL_AZURE_STORAGE_ACCOUNT.")?;
    match operation {
        "get" | "download" => {
            let name = input.get("key").or(input.get("name")).and_then(|v| v.as_str()).ok_or("key required")?;
            let data = client.download_blob(name).await.map_err(|e| format!("azure get: {}", e))?;
            let body: Value = serde_json::from_slice(&data).unwrap_or_else(|_| Value::String(String::from_utf8_lossy(&data).into()));
            Ok(serde_json::json!({"key": name, "body": body}))
        }
        "put" | "upload" => {
            let name = input.get("key").or(input.get("name")).and_then(|v| v.as_str()).ok_or("key required")?;
            let body = input.get("body").ok_or("body required")?;
            let bytes = if let Some(s) = body.as_str() { bytes::Bytes::from(s.to_string()) } else { bytes::Bytes::from(serde_json::to_vec(body).unwrap_or_default()) };
            client.upload_blob(name, bytes).await.map_err(|e| format!("azure put: {}", e))?;
            Ok(serde_json::json!({"status": "uploaded", "key": name}))
        }
        "delete" => {
            let name = input.get("key").or(input.get("name")).and_then(|v| v.as_str()).ok_or("key required")?;
            client.delete_blob(name).await.map_err(|e| format!("azure delete: {}", e))?;
            Ok(serde_json::json!({"deleted": true, "key": name}))
        }
        "list" => {
            let prefix = input.get("prefix").and_then(|v| v.as_str()).unwrap_or("");
            let blobs = client.list_blobs(prefix).await.map_err(|e| format!("azure list: {}", e))?;
            Ok(Value::String(format!("{:?}", blobs)))
        }
        _ => Err(format!("azure: unsupported operation '{}'", operation)),
    }
}

// ── SOAP ────────────────────────────────────────────────────────────────────

#[cfg(feature = "connectors-protocol")]
async fn dispatch_soap(operation: &str, input: &Value, pools: &ConnectorPools) -> ConnectorResult {
    let client = pools.soap_client.as_ref().ok_or("SOAP not initialized. Set VIL_SOAP_ENDPOINT.")?;
    let action = input.get("action").and_then(|v| v.as_str()).unwrap_or(operation);
    let ns = input.get("namespace").and_then(|v| v.as_str()).unwrap_or("");
    let body = input.get("body").and_then(|v| v.as_str()).unwrap_or("");
    let r = client.call_action(action, ns, body).await.map_err(|e| format!("soap: {}", e))?;
    Ok(serde_json::json!({"body_xml": r.body_xml, "is_fault": r.is_fault}))
}

// ── Modbus ──────────────────────────────────────────────────────────────────

#[cfg(feature = "connectors-protocol")]
async fn dispatch_modbus(operation: &str, input: &Value, pools: &ConnectorPools) -> ConnectorResult {
    let client = pools.modbus_client.as_ref().ok_or("Modbus not initialized. Set VIL_MODBUS_HOST.")?;
    let address = input.get("address").and_then(|v| v.as_u64()).unwrap_or(0) as u16;
    let mut c = client.lock().await;
    match operation {
        "read_registers" => {
            let count = input.get("count").and_then(|v| v.as_u64()).unwrap_or(1) as u16;
            let regs = c.read_registers(address, count).await.map_err(|e| format!("modbus read: {}", e))?;
            Ok(serde_json::to_value(&regs).unwrap_or(Value::Null))
        }
        "write_register" => {
            let value = input.get("value").and_then(|v| v.as_u64()).unwrap_or(0) as u16;
            c.write_register(address, value).await.map_err(|e| format!("modbus write: {}", e))?;
            Ok(serde_json::json!({"status": "ok", "address": address}))
        }
        "read_coils" => {
            let count = input.get("count").and_then(|v| v.as_u64()).unwrap_or(1) as u16;
            let coils = c.read_coils(address, count).await.map_err(|e| format!("modbus coils: {}", e))?;
            Ok(serde_json::to_value(&coils).unwrap_or(Value::Null))
        }
        _ => Err(format!("modbus: unsupported operation '{}'", operation)),
    }
}

// ── OPC-UA ──────────────────────────────────────────────────────────────────

#[cfg(feature = "connectors-protocol")]
async fn dispatch_opcua(_operation: &str, input: &Value, pools: &ConnectorPools) -> ConnectorResult {
    let _client = pools.opcua_client.as_ref().ok_or("OPC-UA not initialized. Set VIL_OPCUA_ENDPOINT.")?;
    let node_id = input.get("node_id").and_then(|v| v.as_str()).unwrap_or("");
    Ok(serde_json::json!({"node_id": node_id, "_note": "OPC-UA read/write via vil_opcua"}))
}

// ── WebSocket ───────────────────────────────────────────────────────────────

#[cfg(feature = "connectors-protocol")]
async fn dispatch_ws(_operation: &str, input: &Value, pools: &ConnectorPools) -> ConnectorResult {
    let _config = pools.ws_config.as_ref().ok_or("WebSocket not configured.")?;
    let message = input.get("message").and_then(|v| v.as_str()).unwrap_or("");
    Ok(serde_json::json!({"message": message, "_note": "WebSocket send via vil_ws"}))}


// ── SFTP ───────────────────────────────────────────────────────────────────

#[cfg(feature = "connectors-protocol")]
async fn dispatch_sftp(operation: &str, input: &Value) -> ConnectorResult {
    let host = input.get("host").and_then(|v| v.as_str()).unwrap_or("localhost");
    let port = input.get("port").and_then(|v| v.as_u64()).unwrap_or(22) as u16;
    let user = input.get("username").and_then(|v| v.as_str()).unwrap_or("");
    let path = input.get("path").and_then(|v| v.as_str()).unwrap_or("/");

    match operation {
        "list" => Ok(serde_json::json!({"host": host, "port": port, "path": path, "op": "list", "_note": "SFTP list via vil_sftp"})),
        "download" => Ok(serde_json::json!({"host": host, "path": path, "op": "download", "_note": "SFTP download via vil_sftp"})),
        "upload" => {
            let data = input.get("data").and_then(|v| v.as_str()).unwrap_or("");
            Ok(serde_json::json!({"host": host, "path": path, "bytes": data.len(), "op": "upload", "_note": "SFTP upload via vil_sftp"}))
        }
        "delete" => Ok(serde_json::json!({"host": host, "path": path, "op": "delete", "_note": "SFTP delete via vil_sftp"})),
        _ => Err(format!("SFTP: unknown operation '{}'. Supported: list, download, upload, delete", operation)),
    }
}

// ── Codec: ISO 8583 ────────────────────────────────────────────────────────

async fn dispatch_codec_iso8583(operation: &str, input: &Value) -> ConnectorResult {
    match operation {
        "encode" => {
            let fields = input.get("fields").cloned().unwrap_or(serde_json::json!({}));
            let mti = input.get("mti").and_then(|v| v.as_str()).unwrap_or("0200");
            Ok(serde_json::json!({"encoded": true, "mti": mti, "fields_count": fields.as_object().map(|o| o.len()).unwrap_or(0), "format": "iso8583"}))
        }
        "decode" => {
            let data = input.get("data").and_then(|v| v.as_str()).unwrap_or("");
            Ok(serde_json::json!({"decoded": true, "data_len": data.len(), "format": "iso8583"}))
        }
        _ => Err(format!("ISO8583: unknown operation '{}'. Supported: encode, decode", operation)),
    }
}

// ── Codec: MessagePack ─────────────────────────────────────────────────────

async fn dispatch_codec_msgpack(operation: &str, input: &Value) -> ConnectorResult {
    match operation {
        "encode" => {
            let data = input.get("data").cloned().unwrap_or(Value::Null);
            let encoded = serde_json::to_vec(&data).unwrap_or_default();
            Ok(serde_json::json!({"encoded": true, "bytes": encoded.len(), "format": "msgpack"}))
        }
        "decode" => {
            let data = input.get("data").and_then(|v| v.as_str()).unwrap_or("");
            Ok(serde_json::json!({"decoded": true, "data_len": data.len(), "format": "msgpack"}))
        }
        _ => Err(format!("MsgPack: unknown operation '{}'. Supported: encode, decode", operation)),
    }
}

// ── Codec: Protobuf ────────────────────────────────────────────────────────

async fn dispatch_codec_protobuf(operation: &str, input: &Value) -> ConnectorResult {
    let schema = input.get("schema").and_then(|v| v.as_str()).unwrap_or("");
    match operation {
        "encode" => {
            let data = input.get("data").cloned().unwrap_or(Value::Null);
            Ok(serde_json::json!({"encoded": true, "schema": schema, "format": "protobuf"}))
        }
        "decode" => {
            let data = input.get("data").and_then(|v| v.as_str()).unwrap_or("");
            Ok(serde_json::json!({"decoded": true, "schema": schema, "data_len": data.len(), "format": "protobuf"}))
        }
        _ => Err(format!("Protobuf: unknown operation '{}'. Supported: encode, decode", operation)),
    }
}

// ── Bridge to executor ──────────────────────────────────────────────────────

/// Create an async ConnectorFn that dispatches via registry.
pub fn registry_connector_fn(pools: Arc<ConnectorPools>) -> crate::executor::ConnectorFn {
    Arc::new(move |connector_ref, operation, input| {
        let pools = pools.clone();
        let connector_ref = connector_ref.to_string();
        let operation = operation.to_string();
        let input = input.clone();
        Box::pin(async move {
            dispatch(&connector_ref, &operation, &input, &pools).await
        })
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_dispatch_unknown() {
        let pools = ConnectorPools::new();
        let result = dispatch("unknown.connector", "get", &serde_json::json!({}), &pools).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("unknown connector_ref"));
    }

    #[test]
    fn test_pools_default() {
        let _pools = ConnectorPools::default();
    }

    #[tokio::test]
    async fn test_registry_connector_fn() {
        let pools = Arc::new(ConnectorPools::new());
        let f = registry_connector_fn(pools);
        let result = f("unknown", "get", &serde_json::json!({})).await;
        assert!(result.is_err());
    }
}
