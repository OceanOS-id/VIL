// +============================================================+
// |  607 -- VilORM Multi-Tenant SaaS (SQLite)                 |
// +============================================================+
// |  Pattern:  VilEntity + VilQuery                           |
// |  Features: Tenant-scoped data isolation                   |
// |  Domain:   Tenants, Users, Settings                       |
// +============================================================+
// |  Demonstrates VilORM patterns:                            |
// |  1. #[derive(VilEntity)] on 3 models                     |
// |  2. on_conflict("key").do_update(&["value"]) -- upsert   |
// |  3. on_conflict_nothing("tenant_id, email") -- idempotent |
// |  4. update().set_optional() -- partial tenant update      |
// |  5. select().where_eq().and_eq() -- scoped queries        |
// |  6. select_expr("COUNT(*)").where_eq().scalar::<i64>()    |
// |  7. T::find_where() -- lookup by unique field             |
// +============================================================+
//
// Run:   cargo run -p vil-db-vilorm-multitenant
// Test:
//   # Create tenant
//   curl -X POST http://localhost:8087/api/saas/tenants \
//     -H 'Content-Type: application/json' \
//     -d '{"name":"Acme Corp","plan":"pro"}'
//
//   # Add user to tenant (idempotent)
//   curl -X POST http://localhost:8087/api/saas/tenants/<id>/users \
//     -H 'Content-Type: application/json' \
//     -d '{"email":"alice@acme.com","role":"admin"}'
//
//   # Upsert setting
//   curl -X POST http://localhost:8087/api/saas/tenants/<id>/settings \
//     -H 'Content-Type: application/json' \
//     -d '{"key":"theme","value":"dark"}'
//
//   # Stats
//   curl http://localhost:8087/api/saas/tenants/<id>/stats

use serde::{Deserialize, Serialize};
use std::sync::Arc;
use vil_db_sqlx::SqlxPool;
use vil_orm_derive::VilEntity;
use vil_server::prelude::*;

// -- Models --

#[derive(Debug, Clone, Serialize, Deserialize, VilModel, sqlx::FromRow, VilEntity)]
#[vil_entity(table = "tenants")]
struct Tenant {
    #[vil_entity(pk)]
    id: String,
    name: String,
    plan: String,
    is_active: i64,
    #[vil_entity(auto_now_add)]
    created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, VilModel, sqlx::FromRow, VilEntity)]
#[vil_entity(table = "tenant_users")]
struct TenantUser {
    #[vil_entity(pk)]
    id: String,
    tenant_id: String,
    email: String,
    role: String,
    #[vil_entity(auto_now_add)]
    created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, VilModel, sqlx::FromRow, VilEntity)]
#[vil_entity(table = "tenant_settings")]
struct TenantSetting {
    #[vil_entity(pk)]
    id: String,
    tenant_id: String,
    key: String,
    value: Option<String>,
}

// -- Request types --

#[derive(Debug, Deserialize)]
struct CreateTenant {
    name: String,
    plan: Option<String>,
}

#[derive(Debug, Deserialize)]
struct UpdateTenant {
    name: Option<String>,
    plan: Option<String>,
    is_active: Option<i64>,
}

#[derive(Debug, Deserialize)]
struct AddUser {
    email: String,
    role: Option<String>,
}

#[derive(Debug, Deserialize)]
struct UpsertSetting {
    key: String,
    value: Option<String>,
}

// -- State --

#[derive(Clone)]
struct AppState {
    pool: Arc<SqlxPool>,
}

// -- Handlers --

/// POST /tenants -- create tenant
async fn create_tenant(ctx: ServiceCtx, body: ShmSlice) -> HandlerResult<VilResponse<Tenant>> {
    let state = ctx.state::<AppState>().expect("state");
    let req: CreateTenant = body.json().map_err(|_| VilError::bad_request("invalid JSON"))?;
    let id = uuid::Uuid::new_v4().to_string();
    let plan = req.plan.unwrap_or_else(|| "free".to_string());

    Tenant::q()
        .insert_columns(&["id", "name", "plan", "is_active"])
        .value(id.clone())
        .value(req.name)
        .value(plan)
        .value(1_i64)
        .execute(state.pool.inner())
        .await
        .map_err(|e| VilError::internal(format!("{e}")))?;

    let tenant = Tenant::find_by_id(state.pool.inner(), &id)
        .await
        .map_err(|e| VilError::internal(format!("{e}")))?
        .ok_or_else(|| VilError::internal("created but not found"))?;

    Ok(VilResponse::created(tenant))
}

/// GET /tenants/:id
async fn get_tenant(ctx: ServiceCtx, Path(id): Path<String>) -> HandlerResult<VilResponse<Tenant>> {
    let state = ctx.state::<AppState>().expect("state");

    let tenant = Tenant::find_by_id(state.pool.inner(), &id)
        .await
        .map_err(|e| VilError::internal(format!("{e}")))?
        .ok_or_else(|| VilError::not_found("Tenant not found"))?;

    Ok(VilResponse::ok(tenant))
}

/// PUT /tenants/:id -- partial update with set_optional
async fn update_tenant(
    ctx: ServiceCtx,
    Path(id): Path<String>,
    body: ShmSlice,
) -> HandlerResult<VilResponse<Tenant>> {
    let state = ctx.state::<AppState>().expect("state");
    let req: UpdateTenant = body.json().map_err(|_| VilError::bad_request("invalid JSON"))?;

    // Pattern: update().set_optional() -- partial tenant update, skips None fields
    Tenant::q()
        .update()
        .set_optional("name", req.name.as_deref())
        .set_optional("plan", req.plan.as_deref())
        .set_optional_i64("is_active", req.is_active)
        .where_eq("id", &id)
        .execute(state.pool.inner())
        .await
        .map_err(|e| VilError::internal(format!("{e}")))?;

    let tenant = Tenant::find_by_id(state.pool.inner(), &id)
        .await
        .map_err(|e| VilError::internal(format!("{e}")))?
        .ok_or_else(|| VilError::not_found("Tenant not found"))?;

    Ok(VilResponse::ok(tenant))
}

/// POST /tenants/:id/users -- add user, idempotent (ON CONFLICT DO NOTHING)
async fn add_user(
    ctx: ServiceCtx,
    Path(tenant_id): Path<String>,
    body: ShmSlice,
) -> HandlerResult<VilResponse<serde_json::Value>> {
    let state = ctx.state::<AppState>().expect("state");
    let req: AddUser = body.json().map_err(|_| VilError::bad_request("invalid JSON"))?;
    let id = uuid::Uuid::new_v4().to_string();
    let role = req.role.unwrap_or_else(|| "member".to_string());

    // Pattern: on_conflict_nothing("tenant_id, email") -- idempotent user add
    TenantUser::q()
        .insert_columns(&["id", "tenant_id", "email", "role"])
        .value(id)
        .value(tenant_id.clone())
        .value(req.email.clone())
        .value(role)
        .on_conflict_nothing("tenant_id, email")
        .execute(state.pool.inner())
        .await
        .map_err(|e| VilError::internal(format!("{e}")))?;

    Ok(VilResponse::ok(serde_json::json!({
        "ok": true,
        "tenant_id": tenant_id,
        "email": req.email,
    })))
}

/// GET /tenants/:id/users -- list users scoped to tenant
async fn list_users(
    ctx: ServiceCtx,
    Path(tenant_id): Path<String>,
) -> HandlerResult<VilResponse<Vec<TenantUser>>> {
    let state = ctx.state::<AppState>().expect("state");

    // Pattern: select().where_eq().and_eq() -- scoped queries (tenant isolation)
    // Here we use where_eq for tenant_id scoping
    let users = TenantUser::q()
        .select(&["id", "tenant_id", "email", "role", "created_at"])
        .where_eq("tenant_id", &tenant_id)
        .order_by_desc("created_at")
        .fetch_all::<TenantUser>(state.pool.inner())
        .await
        .map_err(|e| VilError::internal(format!("{e}")))?;

    Ok(VilResponse::ok(users))
}

/// POST /tenants/:id/settings -- upsert key-value setting
async fn upsert_setting(
    ctx: ServiceCtx,
    Path(tenant_id): Path<String>,
    body: ShmSlice,
) -> HandlerResult<VilResponse<serde_json::Value>> {
    let state = ctx.state::<AppState>().expect("state");
    let req: UpsertSetting = body.json().map_err(|_| VilError::bad_request("invalid JSON"))?;
    let id = uuid::Uuid::new_v4().to_string();

    // Pattern: on_conflict("tenant_id, key").do_update(&["value"]) -- upsert settings
    TenantSetting::q()
        .insert_columns(&["id", "tenant_id", "key", "value"])
        .value(id)
        .value(tenant_id.clone())
        .value(req.key.clone())
        .value_opt_str(req.value.clone())
        .on_conflict("tenant_id, key")
        .do_update(&["value"])
        .execute(state.pool.inner())
        .await
        .map_err(|e| VilError::internal(format!("{e}")))?;

    Ok(VilResponse::ok(serde_json::json!({
        "ok": true,
        "tenant_id": tenant_id,
        "key": req.key,
        "value": req.value,
    })))
}

/// GET /tenants/:id/settings -- list settings for tenant
async fn list_settings(
    ctx: ServiceCtx,
    Path(tenant_id): Path<String>,
) -> HandlerResult<VilResponse<Vec<TenantSetting>>> {
    let state = ctx.state::<AppState>().expect("state");

    // Pattern: where_eq() for tenant isolation
    let settings = TenantSetting::q()
        .select(&["id", "tenant_id", "key", "value"])
        .where_eq("tenant_id", &tenant_id)
        .order_by_asc("key")
        .fetch_all::<TenantSetting>(state.pool.inner())
        .await
        .map_err(|e| VilError::internal(format!("{e}")))?;

    Ok(VilResponse::ok(settings))
}

/// GET /tenants/:id/stats -- user count, setting count
async fn tenant_stats(
    ctx: ServiceCtx,
    Path(tenant_id): Path<String>,
) -> HandlerResult<VilResponse<serde_json::Value>> {
    let state = ctx.state::<AppState>().expect("state");
    let pool = state.pool.inner();

    // Verify tenant exists
    let tenant = Tenant::find_by_id(pool, &tenant_id)
        .await
        .map_err(|e| VilError::internal(format!("{e}")))?
        .ok_or_else(|| VilError::not_found("Tenant not found"))?;

    // Pattern: select_expr("COUNT(*)").where_eq().scalar::<i64>() -- count per tenant
    let user_count: i64 = TenantUser::q()
        .select_expr("CAST(COUNT(*) AS INTEGER)")
        .where_eq("tenant_id", &tenant_id)
        .scalar::<i64>(pool)
        .await
        .unwrap_or(0);

    let setting_count: i64 = TenantSetting::q()
        .select_expr("CAST(COUNT(*) AS INTEGER)")
        .where_eq("tenant_id", &tenant_id)
        .scalar::<i64>(pool)
        .await
        .unwrap_or(0);

    Ok(VilResponse::ok(serde_json::json!({
        "tenant_id": tenant_id,
        "tenant_name": tenant.name,
        "plan": tenant.plan,
        "is_active": tenant.is_active == 1,
        "user_count": user_count,
        "setting_count": setting_count,
    })))
}

// -- Main --

#[tokio::main]
async fn main() {
    let pool = SqlxPool::connect("multitenant", vil_db_sqlx::SqlxConfig::sqlite("sqlite:multitenant.db?mode=rwc"))
        .await
        .expect("SQLite connect failed");

    pool.execute_raw(
        "CREATE TABLE IF NOT EXISTS tenants (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            plan TEXT DEFAULT 'free',
            is_active INTEGER DEFAULT 1,
            created_at TEXT DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ','now'))
        );
        CREATE TABLE IF NOT EXISTS tenant_users (
            id TEXT PRIMARY KEY,
            tenant_id TEXT NOT NULL REFERENCES tenants(id),
            email TEXT NOT NULL,
            role TEXT DEFAULT 'member',
            created_at TEXT DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ','now')),
            UNIQUE(tenant_id, email)
        );
        CREATE TABLE IF NOT EXISTS tenant_settings (
            id TEXT PRIMARY KEY,
            tenant_id TEXT NOT NULL REFERENCES tenants(id),
            key TEXT NOT NULL,
            value TEXT,
            UNIQUE(tenant_id, key)
        );"
    )
    .await
    .expect("Migration failed");

    let state = AppState { pool: Arc::new(pool) };

    let saas_svc = ServiceProcess::new("saas")
        .endpoint(Method::POST, "/tenants", post(create_tenant))
        .endpoint(Method::GET, "/tenants/:id", get(get_tenant))
        .endpoint(Method::PUT, "/tenants/:id", put(update_tenant))
        .endpoint(Method::POST, "/tenants/:id/users", post(add_user))
        .endpoint(Method::GET, "/tenants/:id/users", get(list_users))
        .endpoint(Method::POST, "/tenants/:id/settings", post(upsert_setting))
        .endpoint(Method::GET, "/tenants/:id/settings", get(list_settings))
        .endpoint(Method::GET, "/tenants/:id/stats", get(tenant_stats))
        .state(state);

    VilApp::new("vilorm-multitenant")
        .port(8087)
        .observer(true)
        .service(saas_svc)
        .run()
        .await;
}
