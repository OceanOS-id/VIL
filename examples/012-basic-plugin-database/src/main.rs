// ╔════════════════════════════════════════════════════════════╗
// ║  012 — Multi-tenant SaaS Database Plugin                  ║
// ╠════════════════════════════════════════════════════════════╣
// ║  Pattern:  VX_APP                                           ║
// ║  Token:    N/A (HTTP server)                                ║
// ║  Features: ShmSlice, VilResponse                            ║
// ║  Domain:   Database integration for SaaS platform —         ║
// ║            SQLx (PostgreSQL), Redis cache layers             ║
// ╚════════════════════════════════════════════════════════════╝
// basic-usage-plugin-database — Plugin Manager + DB sqlx Patterns (VX Architecture)
// =============================================================================
//
// BUSINESS CONTEXT:
//   Multi-tenant SaaS platform database layer. Tenant data lives in PostgreSQL
//   ("primary" pool), and hot data is served from Redis cache. This separation
//   prevents cache-miss storms from starving transactional workloads.
//
// Prerequisites:
//   docker compose -f examples/docker-compose.yml up -d
//   (starts PostgreSQL on :5432 with vil_demo DB, Redis on :6380)
//   Tables `tasks` and `products` are auto-created by init-postgres.sql.
//
// Environment variables:
//   DATABASE_URL  — PostgreSQL connection (default: postgres://postgres:vilpass@localhost:5432/vil_demo)
//   REDIS_URL     — Redis connection (default: redis://localhost:6380)
//
// Demonstrates vil_db_sqlx plugin with REAL PostgreSQL and Redis connections
// using the VX Process-Oriented architecture (VilApp + ServiceProcess).
//
// VX highlights:
//   - ServiceProcess groups endpoints as a logical Process
//   - VilApp orchestrates processes with Tri-Lane mesh
//   - Handlers stay EXACTLY the same as classic vil-server
//
// Routes:
//   GET  /              → overview page
//   GET  /plugins       → list available DB plugins with status
//   GET  /config        → show database configuration patterns (masked secrets)
//   GET  /products      → query real products table
//   POST /tasks         → create a task in PostgreSQL
//   GET  /tasks         → list all tasks from PostgreSQL
//   GET  /pool-stats    → show REAL connection pool metrics
//   GET  /redis-ping    → ping Redis and show cache demo
//
// Built-in endpoints (auto-provided by VilApp):
//   GET  /health        → health check
//   GET  /ready         → readiness probe
//   GET  /metrics       → Prometheus-style metrics
//   GET  /info          → server info
//
// Run:
//   cargo run -p vil-basic-plugin-database
//
// Test:
//   curl http://localhost:8080/api/plugin-db/
//   curl http://localhost:8080/api/plugin-db/plugins
//   curl http://localhost:8080/api/plugin-db/config
//   curl http://localhost:8080/api/plugin-db/products
//   curl -X POST http://localhost:8080/api/plugin-db/tasks \
//     -H 'Content-Type: application/json' \
//     -d '{"title":"Deploy v2","description":"Deploy version 2 to staging"}'
//   curl http://localhost:8080/api/plugin-db/tasks
//   curl http://localhost:8080/api/plugin-db/pool-stats
//   curl http://localhost:8080/api/plugin-db/redis-ping
// =============================================================================

use vil_db_sqlx::{SqlxConfig, SqlxPool};
use vil_orm::VilQuery;
use vil_orm_derive::VilEntity;
use vil_server::prelude::*;

use std::sync::Arc;

// ---------------------------------------------------------------------------
// Domain models — typed response structs
// ---------------------------------------------------------------------------

/// Response for GET /api/plugins.
#[derive(Clone, Debug, Serialize, Deserialize, VilModel)]
struct PluginListResponse {
    plugins: Vec<PluginInfo>,
    total_plugins: usize,
}

#[derive(Clone, Debug, Serialize, Deserialize, VilModel)]
struct PluginInfo {
    name: String,
    version: String,
    description: String,
    status: String,
    features: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    supported_drivers: Option<Vec<String>>,
    manifest: PluginManifest,
}

#[derive(Clone, Debug, Serialize, Deserialize, VilModel)]
struct PluginManifest {
    plugin_type: String,
    admin_ui: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    config_schema: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    health_endpoint: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    metrics_endpoint: Option<String>,
}

/// Response for GET /api/config.
#[derive(Clone, Debug, Serialize, Deserialize, VilModel)]
struct ConfigResponse {
    database_configs: Vec<DatabaseConfigInfo>,
    pool_count: usize,
    redis_url: String,
    note: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, VilModel)]
struct DatabaseConfigInfo {
    pool_name: String,
    driver: String,
    url: String,
    max_connections: u32,
    min_connections: u32,
    connect_timeout_secs: u64,
    idle_timeout_secs: u64,
    ssl_mode: String,
    services: Vec<String>,
}

/// Response for GET /api/products.
#[derive(Clone, Debug, Serialize, Deserialize, VilModel)]
struct ProductsResponse {
    products: Vec<ProductRow>,
    count: usize,
    source: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, VilModel, sqlx::FromRow, VilEntity)]
#[vil_entity(table = "products")]
struct ProductRow {
    #[vil_entity(pk)]
    id: i32,
    name: String,
    category: String,
    price: f64,
    stock: i32,
}

/// Request body for POST /api/tasks.
#[derive(Debug, Clone, Deserialize, Serialize, VilModel)]
struct CreateTaskRequest {
    title: String,
    #[serde(default)]
    description: Option<String>,
}

/// Response for POST /api/tasks.
#[derive(Clone, Debug, Serialize, Deserialize, VilModel)]
struct CreateTaskResponse {
    id: i32,
    title: String,
    description: String,
    done: bool,
    message: String,
}

/// Response for GET /api/tasks.
#[derive(Clone, Debug, Serialize, Deserialize, VilModel)]
struct TasksResponse {
    tasks: Vec<TaskRow>,
    count: usize,
    source: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, VilModel, sqlx::FromRow, VilEntity)]
#[vil_entity(table = "tasks")]
struct TaskRow {
    #[vil_entity(pk)]
    id: i32,
    title: String,
    description: String,
    done: bool,
}

/// Response for GET /api/pool-stats.
#[derive(Clone, Debug, Serialize, Deserialize, VilModel)]
struct PoolStatsResponse {
    pools: std::collections::HashMap<String, PoolDetail>,
    total_pools: usize,
    is_demo: bool,
    prometheus_hint: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, VilModel)]
struct PoolDetail {
    driver: String,
    size: PoolSize,
    metrics: PoolMetricsInfo,
    status: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, VilModel)]
struct PoolSize {
    max: u32,
    min: u32,
    current: u32,
    idle: u32,
}

#[derive(Clone, Debug, Serialize, Deserialize, VilModel)]
struct PoolMetricsInfo {
    queries_total: u64,
    query_errors: u64,
    avg_query_us: u64,
    acquires_total: u64,
    health_checks_ok: u64,
    health_checks_fail: u64,
}

/// Response for GET /api/redis-ping.
#[derive(Clone, Debug, Serialize, Deserialize, VilModel)]
struct RedisPingResponse {
    connected: bool,
    pong: String,
    cache_demo: CacheDemo,
}

#[derive(Clone, Debug, Serialize, Deserialize, VilModel)]
struct CacheDemo {
    key: String,
    value_written: String,
    value_read: String,
    ttl_secs: i64,
}

// ---------------------------------------------------------------------------
// Shared state — holds real DB pool + Redis connection
// ---------------------------------------------------------------------------

#[derive(Clone)]
struct DbState {
    pg_pool: Arc<SqlxPool>,
    pg_config: SqlxConfig,
    redis_client: redis::Client,
    redis_url: String,
}

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

/// GET / — overview of the plugin database example.
async fn index() -> &'static str {
    "VIL Plugin Database Example (REAL connections)\n\
     ====================================================\n\n\
     Connected to real PostgreSQL and Redis.\n\n\
     Endpoints:\n\
     - GET  /api/plugin-db/plugins    — available DB plugins and their status\n\
     - GET  /api/plugin-db/config     — database configuration (secrets masked)\n\
     - GET  /api/plugin-db/products   — query real products table\n\
     - POST /api/plugin-db/tasks      — create a task (real INSERT)\n\
     - GET  /api/plugin-db/tasks      — list all tasks (real SELECT)\n\
     - GET  /api/plugin-db/pool-stats — REAL connection pool metrics\n\
     - GET  /api/plugin-db/redis-ping — ping Redis + cache demo\n\n\
     Built-in:\n\
     - GET  /health, /ready, /metrics, /info\n"
}

/// GET /api/plugins — list available database plugins with status info.
async fn list_plugins(ctx: ServiceCtx) -> VilResponse<PluginListResponse> {
    let state = ctx.state::<DbState>().expect("state type mismatch");

    // Check real connectivity for status
    let pg_status = if state.pg_pool.inner().size() > 0 {
        "connected"
    } else {
        "disconnected"
    };

    let redis_status = match redis::Client::open(state.redis_url.as_str()) {
        Ok(_) => "available",
        Err(_) => "unavailable",
    };

    let plugins = vec![
        PluginInfo {
            name: "vil_db_sqlx".to_string(),
            version: "0.1.0".to_string(),
            description: "sqlx connection pool — PostgreSQL (REAL connection)".to_string(),
            status: pg_status.to_string(),
            features: vec![
                "Multi-pool manager (per-service pools)".into(),
                "Connection metrics (queries, latency, errors)".into(),
                "Health check integration".into(),
                "Config hot-reload".into(),
                "DbConn extractor for handlers".into(),
            ],
            supported_drivers: Some(vec!["postgres".into(), "mysql".into(), "sqlite".into()]),
            manifest: PluginManifest {
                plugin_type: "database".to_string(),
                admin_ui: true,
                config_schema: Some("SqlxConfig".to_string()),
                health_endpoint: Some("/health/db".to_string()),
                metrics_endpoint: Some("/metrics/db".to_string()),
            },
        },
        PluginInfo {
            name: "vil_db_redis".to_string(),
            version: "0.1.0".to_string(),
            description: "Redis adapter — caching, sessions, pub/sub (REAL connection)".to_string(),
            status: redis_status.to_string(),
            features: vec![
                "Connection pool".into(),
                "Pub/Sub support".into(),
                "Cluster mode".into(),
                "Session store".into(),
            ],
            supported_drivers: None,
            manifest: PluginManifest {
                plugin_type: "cache".to_string(),
                admin_ui: true,
                config_schema: None,
                health_endpoint: None,
                metrics_endpoint: None,
            },
        },
    ];

    VilResponse::ok(PluginListResponse {
        total_plugins: plugins.len(),
        plugins,
    })
}

/// GET /api/config — show database configuration patterns with masked secrets.
async fn show_config(ctx: ServiceCtx) -> VilResponse<ConfigResponse> {
    let state = ctx.state::<DbState>().expect("state type mismatch");

    // Mask the password in the URL for display
    let masked_url = mask_url(&state.pg_config.url);
    let masked_redis = mask_url(&state.redis_url);

    let configs = vec![DatabaseConfigInfo {
        pool_name: "primary".to_string(),
        driver: state.pg_config.driver.clone(),
        url: masked_url,
        max_connections: state.pg_config.max_connections,
        min_connections: state.pg_config.min_connections,
        connect_timeout_secs: state.pg_config.connect_timeout_secs,
        idle_timeout_secs: state.pg_config.idle_timeout_secs,
        ssl_mode: state.pg_config.ssl_mode.clone(),
        services: state.pg_config.services.clone(),
    }];

    VilResponse::ok(ConfigResponse {
        pool_count: configs.len(),
        database_configs: configs,
        redis_url: masked_redis,
        note: "REAL connections — PostgreSQL and Redis are live.".to_string(),
    })
}

/// GET /api/products — query products via VilQuery (specific columns, not SELECT *)
async fn list_products(ctx: ServiceCtx) -> HandlerResult<VilResponse<ProductsResponse>> {
    let state = ctx.state::<DbState>().expect("state type mismatch");

    let products = ProductRow::q()
        .select(&["id", "name", "category", "price::float8 as price", "stock"])
        .order_by_asc("id")
        .fetch_all::<ProductRow>(state.pg_pool.inner())
        .await
        .map_err(|e| VilError::internal(format!("PostgreSQL query failed: {}", e)))?;

    let count = products.len();
    Ok(VilResponse::ok(ProductsResponse {
        products,
        count,
        source: "PostgreSQL (vil_demo.products) via VilQuery".to_string(),
    }))
}

/// POST /api/tasks — create a new task via VilQuery insert
async fn create_task(
    ctx: ServiceCtx,
    body: ShmSlice,
) -> HandlerResult<VilResponse<CreateTaskResponse>> {
    let state = ctx.state::<DbState>().expect("state type mismatch");
    let req: CreateTaskRequest = body.json().expect("invalid JSON body");

    if req.title.trim().is_empty() {
        return Err(VilError::bad_request("title must not be empty"));
    }

    let title = req.title;
    let description = req.description.unwrap_or_default();

    // PostgreSQL RETURNING via raw query (VilQuery doesn't support RETURNING yet)
    // But we showcase VilQuery insert for the write path:
    TaskRow::q()
        .insert_columns(&["title", "description"])
        .value(title.clone())
        .value(description.clone())
        .execute(state.pg_pool.inner())
        .await
        .map_err(|e| VilError::internal(format!("PostgreSQL insert failed: {}", e)))?;

    // Fetch back via VilQuery
    let task = TaskRow::q()
        .select(&["id", "title", "description", "done"])
        .where_eq("title", &title)
        .order_by_desc("id")
        .limit(1)
        .fetch_optional::<TaskRow>(state.pg_pool.inner())
        .await
        .map_err(|e| VilError::internal(format!("{e}")))?
        .ok_or_else(|| VilError::internal("insert succeeded but fetch failed"))?;

    Ok(VilResponse::ok(CreateTaskResponse {
        id: task.id,
        title: task.title,
        description: task.description,
        done: task.done,
        message: "Task created in PostgreSQL via VilQuery".to_string(),
    }))
}

/// GET /api/tasks — list all tasks via VilQuery (specific columns)
async fn list_tasks(ctx: ServiceCtx) -> HandlerResult<VilResponse<TasksResponse>> {
    let state = ctx.state::<DbState>().expect("state type mismatch");

    let tasks = TaskRow::q()
        .select(&["id", "title", "description", "done"])
        .order_by_asc("id")
        .fetch_all::<TaskRow>(state.pg_pool.inner())
        .await
        .map_err(|e| VilError::internal(format!("PostgreSQL query failed: {}", e)))?;

    let count = tasks.len();
    Ok(VilResponse::ok(TasksResponse {
        tasks,
        count,
        source: "PostgreSQL (vil_demo.tasks) via VilQuery".to_string(),
    }))
}

/// GET /api/pool-stats — show REAL connection pool metrics from sqlx.
/// Business: ops team monitors pool saturation to trigger autoscaling.
/// If primary.current approaches primary.max, it signals need for more pods.
async fn pool_stats(ctx: ServiceCtx) -> VilResponse<PoolStatsResponse> {
    let state = ctx.state::<DbState>().expect("state type mismatch");
    let mut pools = std::collections::HashMap::new();

    let size_info = state.pg_pool.size_info();
    let metrics_snap = state.pg_pool.metrics().snapshot();

    pools.insert(
        "primary".to_string(),
        PoolDetail {
            driver: "postgres".to_string(),
            size: PoolSize {
                max: size_info.max,
                min: size_info.min,
                current: size_info.current,
                idle: size_info.idle,
            },
            metrics: PoolMetricsInfo {
                queries_total: metrics_snap.queries_total,
                query_errors: metrics_snap.query_errors,
                avg_query_us: metrics_snap.avg_query_us,
                acquires_total: metrics_snap.acquires_total,
                health_checks_ok: metrics_snap.health_checks_ok,
                health_checks_fail: metrics_snap.health_checks_fail,
            },
            status: "healthy".to_string(),
        },
    );

    VilResponse::ok(PoolStatsResponse {
        total_pools: pools.len(),
        pools,
        is_demo: false,
        prometheus_hint: "GET /metrics for Prometheus-format pool metrics".to_string(),
    })
}

/// GET /api/redis-ping — ping Redis and demonstrate cache set/get.
async fn redis_ping(ctx: ServiceCtx) -> HandlerResult<VilResponse<RedisPingResponse>> {
    let state = ctx.state::<DbState>().expect("state type mismatch");

    let mut conn = state
        .redis_client
        .get_multiplexed_async_connection()
        .await
        .map_err(|e| VilError::internal(format!("Redis connection failed: {}", e)))?;

    // PING
    let pong: String = redis::cmd("PING")
        .query_async(&mut conn)
        .await
        .map_err(|e| VilError::internal(format!("Redis PING failed: {}", e)))?;

    // Cache demo: SET with TTL, then GET
    let cache_key = "vil:demo:greeting";
    let cache_value = "Hello from VIL + Redis!";

    let _: () = redis::cmd("SET")
        .arg(cache_key)
        .arg(cache_value)
        .arg("EX")
        .arg(60)
        .query_async(&mut conn)
        .await
        .map_err(|e| VilError::internal(format!("Redis SET failed: {}", e)))?;

    let read_back: String = redis::cmd("GET")
        .arg(cache_key)
        .query_async(&mut conn)
        .await
        .map_err(|e| VilError::internal(format!("Redis GET failed: {}", e)))?;

    Ok(VilResponse::ok(RedisPingResponse {
        connected: true,
        pong,
        cache_demo: CacheDemo {
            key: cache_key.to_string(),
            value_written: cache_value.to_string(),
            value_read: read_back,
            ttl_secs: 60,
        },
    }))
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Mask password in a connection URL for safe display.
fn mask_url(url: &str) -> String {
    // Simple masking: replace password between : and @ with ********
    if let Some(at_pos) = url.find('@') {
        if let Some(colon_pos) = url[..at_pos].rfind(':') {
            // Check there's a :// before this colon (i.e., protocol separator)
            if let Some(proto_end) = url.find("://") {
                if colon_pos > proto_end + 3 {
                    return format!("{}:********{}", &url[..colon_pos], &url[at_pos..]);
                }
            }
        }
    }
    url.to_string()
}

// ---------------------------------------------------------------------------
// Main
// ---------------------------------------------------------------------------

#[tokio::main]
async fn main() {
    // Read connection URLs from environment with sensible defaults
    let database_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://postgres:vilpass@localhost:5432/vil_demo".to_string());
    let redis_url =
        std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://localhost:6380".to_string());

    println!("Connecting to PostgreSQL: {}", mask_url(&database_url));
    println!("Connecting to Redis: {}", redis_url);

    // Connect to PostgreSQL via vil_db_sqlx (REAL sqlx pool)
    let pg_config = SqlxConfig::postgres(&database_url)
        .max_connections(10)
        .min_connections(2)
        .timeout(5);

    let pg_pool = SqlxPool::connect("primary", pg_config.clone())
        .await
        .expect("Failed to connect to PostgreSQL — is it running? (docker compose -f examples/docker-compose.yml up -d)");

    // Verify PostgreSQL connectivity
    pg_pool
        .execute_raw("SELECT 1")
        .await
        .expect("PostgreSQL health check failed");
    println!("PostgreSQL connected successfully.");

    // Connect to Redis (using redis crate directly — vil_db_redis is a facade)
    let redis_client = redis::Client::open(redis_url.as_str())
        .expect("Failed to create Redis client — check REDIS_URL");

    // Verify Redis connectivity
    {
        let mut conn = redis_client.get_multiplexed_async_connection().await
            .expect("Failed to connect to Redis — is it running? (docker compose -f examples/docker-compose.yml up -d)");
        let pong: String = redis::cmd("PING")
            .query_async(&mut conn)
            .await
            .expect("Redis PING failed");
        println!("Redis connected: {}", pong);
    }

    let state = DbState {
        pg_pool: Arc::new(pg_pool),
        pg_config,
        redis_client,
        redis_url,
    };

    // VX: Define plugin-db service as a Process
    let plugin_db_service = ServiceProcess::new("plugin-db")
        .endpoint(Method::GET, "/", get(index))
        .endpoint(Method::GET, "/plugins", get(list_plugins))
        .endpoint(Method::GET, "/config", get(show_config))
        .endpoint(Method::GET, "/products", get(list_products))
        .endpoint(Method::POST, "/tasks", post(create_task))
        .endpoint(Method::GET, "/tasks", get(list_tasks))
        .endpoint(Method::GET, "/pool-stats", get(pool_stats))
        .endpoint(Method::GET, "/redis-ping", get(redis_ping))
        .state(state);

    // VX: Run as Process-Oriented app
    VilApp::new("plugin-database")
        .port(8080)
        .service(plugin_db_service)
        .run()
        .await;
}
