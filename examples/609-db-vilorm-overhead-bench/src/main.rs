// 609 — VilORM vs Raw sqlx Overhead Benchmark
// Identical queries, side-by-side endpoints.
// Compare: /api/bench/raw/* vs /api/bench/orm/*

use serde::{Deserialize, Serialize};
use std::sync::Arc;
use vil_db_sqlx::SqlxPool;
use vil_orm::VilQuery;
use vil_orm_derive::VilEntity;
use vil_server::prelude::*;

#[derive(Debug, Clone, Serialize, Deserialize, VilModel, sqlx::FromRow, VilEntity)]
#[vil_entity(table = "items")]
struct Item {
    #[vil_entity(pk)]
    id: String,
    name: String,
    value: i64,
    created_at: String,
}

#[derive(Clone)]
struct AppState {
    pool: Arc<SqlxPool>,
}

// ── RAW sqlx handlers (baseline) ──

async fn raw_find_by_id(ctx: ServiceCtx, Path(id): Path<String>) -> HandlerResult<VilResponse<Item>> {
    let state = ctx.state::<AppState>().expect("state");
    let item = sqlx::query_as::<_, Item>("SELECT * FROM items WHERE id = $1")
        .bind(&id)
        .fetch_optional(state.pool.inner())
        .await
        .map_err(|e| VilError::internal(format!("{e}")))?
        .ok_or_else(|| VilError::not_found("not found"))?;
    Ok(VilResponse::ok(item))
}

async fn raw_list(ctx: ServiceCtx) -> HandlerResult<VilResponse<Vec<Item>>> {
    let state = ctx.state::<AppState>().expect("state");
    let items = sqlx::query_as::<_, Item>("SELECT * FROM items ORDER BY id DESC LIMIT 100")
        .fetch_all(state.pool.inner())
        .await
        .map_err(|e| VilError::internal(format!("{e}")))?;
    Ok(VilResponse::ok(items))
}

async fn raw_count(ctx: ServiceCtx) -> HandlerResult<VilResponse<serde_json::Value>> {
    let state = ctx.state::<AppState>().expect("state");
    let count: i64 = sqlx::query_scalar("SELECT CAST(COUNT(*) AS INTEGER) FROM items")
        .fetch_one(state.pool.inner())
        .await
        .map_err(|e| VilError::internal(format!("{e}")))?;
    Ok(VilResponse::ok(serde_json::json!({"count": count})))
}

async fn raw_select_cols(ctx: ServiceCtx) -> HandlerResult<VilResponse<Vec<(String, i64)>>> {
    let state = ctx.state::<AppState>().expect("state");
    let rows = sqlx::query_as::<_, (String, i64)>("SELECT name, value FROM items ORDER BY value DESC LIMIT 20")
        .fetch_all(state.pool.inner())
        .await
        .map_err(|e| VilError::internal(format!("{e}")))?;
    Ok(VilResponse::ok(rows))
}

// ── VilORM handlers (same queries via VilEntity + VilQuery) ──

async fn orm_find_by_id(ctx: ServiceCtx, Path(id): Path<String>) -> HandlerResult<VilResponse<Item>> {
    let state = ctx.state::<AppState>().expect("state");
    let item = Item::find_by_id(state.pool.inner(), &id)
        .await
        .map_err(|e| VilError::internal(format!("{e}")))?
        .ok_or_else(|| VilError::not_found("not found"))?;
    Ok(VilResponse::ok(item))
}

async fn orm_list(ctx: ServiceCtx) -> HandlerResult<VilResponse<Vec<Item>>> {
    let state = ctx.state::<AppState>().expect("state");
    let items = Item::find_all(state.pool.inner())
        .await
        .map_err(|e| VilError::internal(format!("{e}")))?;
    Ok(VilResponse::ok(items))
}

async fn orm_count(ctx: ServiceCtx) -> HandlerResult<VilResponse<serde_json::Value>> {
    let state = ctx.state::<AppState>().expect("state");
    let count = Item::count(state.pool.inner())
        .await
        .map_err(|e| VilError::internal(format!("{e}")))?;
    Ok(VilResponse::ok(serde_json::json!({"count": count})))
}

async fn orm_select_cols(ctx: ServiceCtx) -> HandlerResult<VilResponse<Vec<(String, i64)>>> {
    let state = ctx.state::<AppState>().expect("state");
    let rows = Item::q()
        .select(&["name", "value"])
        .order_by_desc("value")
        .limit(20)
        .fetch_all::<(String, i64)>(state.pool.inner())
        .await
        .map_err(|e| VilError::internal(format!("{e}")))?;
    Ok(VilResponse::ok(rows))
}

#[tokio::main]
async fn main() {
    let pool = SqlxPool::connect("bench", vil_db_sqlx::SqlxConfig::sqlite("sqlite:bench.db?mode=rwc"))
        .await.expect("SQLite connect failed");

    pool.execute_raw(
        "CREATE TABLE IF NOT EXISTS items (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            value INTEGER DEFAULT 0,
            created_at TEXT DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ','now'))
        )"
    ).await.expect("Migration failed");

    // Seed 100 rows for benchmark
    for i in 0..100 {
        let id = format!("item-{:04}", i);
        let name = format!("Item {}", i);
        let _ = sqlx::query("INSERT OR IGNORE INTO items (id, name, value) VALUES ($1, $2, $3)")
            .bind(&id).bind(&name).bind(i as i64)
            .execute(pool.inner()).await;
    }

    let state = AppState { pool: Arc::new(pool) };

    let bench_svc = ServiceProcess::new("bench")
        // Raw sqlx
        .endpoint(Method::GET, "/raw/items/:id", get(raw_find_by_id))
        .endpoint(Method::GET, "/raw/items", get(raw_list))
        .endpoint(Method::GET, "/raw/count", get(raw_count))
        .endpoint(Method::GET, "/raw/cols", get(raw_select_cols))
        // VilORM
        .endpoint(Method::GET, "/orm/items/:id", get(orm_find_by_id))
        .endpoint(Method::GET, "/orm/items", get(orm_list))
        .endpoint(Method::GET, "/orm/count", get(orm_count))
        .endpoint(Method::GET, "/orm/cols", get(orm_select_cols))
        .state(state);

    VilApp::new("overhead-bench")
        .port(8099)
        .service(bench_svc)
        .run().await;
}
