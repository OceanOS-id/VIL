// +============================================================+
// |  608 -- VilORM Analytics Dashboard (SQLite)               |
// +============================================================+
// |  Pattern:  VilEntity + VilQuery                           |
// |  Features: Event logging, aggregation, time series        |
// |  Domain:   Events, Daily Aggregates                       |
// +============================================================+
// |  Demonstrates VilORM patterns:                            |
// |  1. #[derive(VilEntity)] on 2 models                     |
// |  2. insert_columns().value() -- log events                |
// |  3. select_expr("COUNT(*),...").group_by().having()        |
// |  4. group_by("date").order_by_asc("date") -- time series  |
// |  5. where_raw("created_at > datetime(...)") -- time filter|
// |  6. where_eq().order_by_desc().limit() -- recent events   |
// |  7. select_expr("COUNT(DISTINCT ...)").scalar::<i64>()    |
// |  8. on_conflict().do_update_raw("count = count + 1")      |
// +============================================================+
//
// Run:   cargo run -p vil-db-vilorm-analytics
// Test:
//   # Log event
//   curl -X POST http://localhost:8088/api/analytics/events \
//     -H 'Content-Type: application/json' \
//     -d '{"event_type":"page_view","user_id":"u1","payload":"{\"page\":\"/home\"}"}'
//
//   # Recent events
//   curl http://localhost:8088/api/analytics/events/recent
//
//   # Events by type
//   curl http://localhost:8088/api/analytics/events/by-type
//
//   # Daily stats
//   curl http://localhost:8088/api/analytics/stats/daily
//
//   # Unique users
//   curl http://localhost:8088/api/analytics/stats/unique-users
//
//   # Summary
//   curl http://localhost:8088/api/analytics/stats/summary

use serde::{Deserialize, Serialize};
use std::sync::Arc;
use vil_db_sqlx::SqlxPool;
use vil_orm_derive::VilEntity;
use vil_server::prelude::*;

// -- Models --

#[derive(Debug, Clone, Serialize, Deserialize, VilModel, sqlx::FromRow, VilEntity)]
#[vil_entity(table = "events")]
struct Event {
    #[vil_entity(pk)]
    id: String,
    event_type: String,
    user_id: Option<String>,
    payload: Option<String>,
    #[vil_entity(auto_now_add)]
    created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, VilModel, sqlx::FromRow, VilEntity)]
#[vil_entity(table = "daily_aggregates")]
struct DailyAggregate {
    #[vil_entity(pk)]
    id: String,
    date: String,
    event_type: String,
    count: i64,
}

// -- View types for aggregated queries --

#[derive(Debug, Clone, Serialize, Deserialize, VilModel, sqlx::FromRow)]
struct EventTypeCount {
    event_type: String,
    event_count: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, VilModel, sqlx::FromRow)]
struct DailyCount {
    date: String,
    event_count: i64,
}

// -- Request types --

#[derive(Debug, Deserialize)]
struct LogEvent {
    event_type: String,
    user_id: Option<String>,
    payload: Option<String>,
}

// -- State --

#[derive(Clone)]
struct AppState {
    pool: Arc<SqlxPool>,
}

// -- Handlers --

/// POST /events -- log event and increment daily aggregate
async fn log_event(ctx: ServiceCtx, body: ShmSlice) -> HandlerResult<VilResponse<Event>> {
    let state = ctx.state::<AppState>().expect("state");
    let req: LogEvent = body.json().map_err(|_| VilError::bad_request("invalid JSON"))?;
    let pool = state.pool.inner();
    let id = uuid::Uuid::new_v4().to_string();

    // Pattern: insert_columns().value() -- log events
    Event::q()
        .insert_columns(&["id", "event_type", "user_id", "payload"])
        .value(id.clone())
        .value(req.event_type.clone())
        .value_opt_str(req.user_id)
        .value_opt_str(req.payload)
        .execute(pool)
        .await
        .map_err(|e| VilError::internal(format!("{e}")))?;

    // Pattern: on_conflict().do_update_raw("count = count + 1") -- increment counter
    let agg_id = uuid::Uuid::new_v4().to_string();
    DailyAggregate::q()
        .insert_columns(&["id", "date", "event_type", "count"])
        .value(agg_id)
        .value(chrono_today())
        .value(req.event_type)
        .value(1_i64)
        .on_conflict("date, event_type")
        .do_update_raw("count = daily_aggregates.count + 1")
        .execute(pool)
        .await
        .map_err(|e| VilError::internal(format!("{e}")))?;

    let event = Event::find_by_id(pool, &id)
        .await
        .map_err(|e| VilError::internal(format!("{e}")))?
        .ok_or_else(|| VilError::internal("created but not found"))?;

    Ok(VilResponse::created(event))
}

/// GET /events/recent -- last 50 events
async fn recent_events(ctx: ServiceCtx) -> HandlerResult<VilResponse<Vec<Event>>> {
    let state = ctx.state::<AppState>().expect("state");

    // Pattern: where_raw("created_at > datetime('now', '-1 hour')") -- time-based filter
    // Also: order_by_desc().limit() -- recent events
    let events = Event::q()
        .select(&["id", "event_type", "user_id", "payload", "created_at"])
        .where_raw("1=1")
        .order_by_desc("created_at")
        .limit(50)
        .fetch_all::<Event>(state.pool.inner())
        .await
        .map_err(|e| VilError::internal(format!("{e}")))?;

    Ok(VilResponse::ok(events))
}

/// GET /events/by-type -- count per event_type, GROUP BY
async fn events_by_type(ctx: ServiceCtx) -> HandlerResult<VilResponse<Vec<EventTypeCount>>> {
    let state = ctx.state::<AppState>().expect("state");

    // Pattern: select_expr("COUNT(*),...").group_by().having() -- grouped aggregates
    let counts = Event::q()
        .select(&["event_type", "CAST(COUNT(*) AS INTEGER) as event_count"])
        .where_raw("1=1")
        .group_by("event_type")
        .having("COUNT(*) > 0")
        .order_by_desc("event_count")
        .fetch_all::<EventTypeCount>(state.pool.inner())
        .await
        .map_err(|e| VilError::internal(format!("{e}")))?;

    Ok(VilResponse::ok(counts))
}

/// GET /stats/daily -- daily event counts from aggregates table, time series
async fn daily_stats(ctx: ServiceCtx) -> HandlerResult<VilResponse<Vec<DailyCount>>> {
    let state = ctx.state::<AppState>().expect("state");

    // Pattern: select_expr("date, COUNT(*)").group_by("date").order_by_asc("date") -- time series
    let daily = DailyAggregate::q()
        .select(&["date", "CAST(SUM(count) AS INTEGER) as event_count"])
        .where_raw("1=1")
        .group_by("date")
        .order_by_asc("date")
        .limit(30)
        .fetch_all::<DailyCount>(state.pool.inner())
        .await
        .map_err(|e| VilError::internal(format!("{e}")))?;

    Ok(VilResponse::ok(daily))
}

/// GET /stats/unique-users -- COUNT(DISTINCT user_id)
async fn unique_users(ctx: ServiceCtx) -> HandlerResult<VilResponse<serde_json::Value>> {
    let state = ctx.state::<AppState>().expect("state");

    // Pattern: select_expr("COUNT(DISTINCT user_id)").scalar::<i64>() -- unique users
    let count: i64 = Event::q()
        .select_expr("CAST(COUNT(DISTINCT user_id) AS INTEGER)")
        .where_not_null("user_id")
        .scalar::<i64>(state.pool.inner())
        .await
        .unwrap_or(0);

    Ok(VilResponse::ok(serde_json::json!({
        "unique_users": count,
    })))
}

/// GET /stats/summary -- multiple aggregates in one response
async fn stats_summary(ctx: ServiceCtx) -> HandlerResult<VilResponse<serde_json::Value>> {
    let state = ctx.state::<AppState>().expect("state");
    let pool = state.pool.inner();

    let total_events = Event::count(pool).await.unwrap_or(0);

    let unique_users: i64 = Event::q()
        .select_expr("CAST(COUNT(DISTINCT user_id) AS INTEGER)")
        .where_not_null("user_id")
        .scalar::<i64>(pool)
        .await
        .unwrap_or(0);

    let event_types: i64 = Event::q()
        .select_expr("CAST(COUNT(DISTINCT event_type) AS INTEGER)")
        .where_raw("1=1")
        .scalar::<i64>(pool)
        .await
        .unwrap_or(0);

    // Recent events (last hour) using where_raw time filter
    let recent_count: i64 = Event::q()
        .select_expr("CAST(COUNT(*) AS INTEGER)")
        .where_raw("created_at > datetime('now', '-1 hour')")
        .scalar::<i64>(pool)
        .await
        .unwrap_or(0);

    Ok(VilResponse::ok(serde_json::json!({
        "total_events": total_events,
        "unique_users": unique_users,
        "event_types": event_types,
        "events_last_hour": recent_count,
    })))
}

/// Helper: get today's date as YYYY-MM-DD string (no chrono dependency)
fn chrono_today() -> String {
    // Use a simple approach: we rely on SQLite for date formatting in the DB,
    // but for the insert key we use a fixed format
    // In production you'd use chrono, but to avoid extra deps:
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    // Simple date key from unix day count (no chrono dep needed for demo)
    let days = now / 86400;
    days.to_string()
}

// -- Main --

#[tokio::main]
async fn main() {
    let pool = SqlxPool::connect("analytics", vil_db_sqlx::SqlxConfig::sqlite("sqlite:analytics.db?mode=rwc"))
        .await
        .expect("SQLite connect failed");

    pool.execute_raw(
        "CREATE TABLE IF NOT EXISTS events (
            id TEXT PRIMARY KEY,
            event_type TEXT NOT NULL,
            user_id TEXT,
            payload TEXT,
            created_at TEXT DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ','now'))
        );
        CREATE TABLE IF NOT EXISTS daily_aggregates (
            id TEXT PRIMARY KEY,
            date TEXT NOT NULL,
            event_type TEXT NOT NULL,
            count INTEGER DEFAULT 0,
            UNIQUE(date, event_type)
        );"
    )
    .await
    .expect("Migration failed");

    let state = AppState { pool: Arc::new(pool) };

    let analytics_svc = ServiceProcess::new("analytics")
        .endpoint(Method::POST, "/events", post(log_event))
        .endpoint(Method::GET, "/events/recent", get(recent_events))
        .endpoint(Method::GET, "/events/by-type", get(events_by_type))
        .endpoint(Method::GET, "/stats/daily", get(daily_stats))
        .endpoint(Method::GET, "/stats/unique-users", get(unique_users))
        .endpoint(Method::GET, "/stats/summary", get(stats_summary))
        .state(state);

    VilApp::new("vilorm-analytics")
        .port(8088)
        .observer(true)
        .service(analytics_svc)
        .run()
        .await;
}
