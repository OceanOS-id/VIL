// ╔════════════════════════════════════════════════════════════╗
// ║  605 — VilORM Complete Showcase (SQLite)                  ║
// ╠════════════════════════════════════════════════════════════╣
// ║  Pattern:  VilEntity + VilQuery                             ║
// ║  Features: All VilORM patterns in one example               ║
// ║  Domain:   Blog platform — posts, authors, tags             ║
// ╠════════════════════════════════════════════════════════════╣
// ║  Demonstrates every VilORM pattern:                         ║
// ║  1. #[derive(VilEntity)] — auto CRUD methods                ║
// ║  2. T::find_by_id() — simple PK lookup                     ║
// ║  3. T::q().select() — column projection (no SELECT *)       ║
// ║  4. T::q().join() — cross-table queries                     ║
// ║  5. T::q().insert_columns().value_opt_str() — NULL-safe     ║
// ║  6. T::q().update().set_optional() — partial update         ║
// ║  7. T::q().on_conflict().do_update() — upsert               ║
// ║  8. T::q().select_expr().scalar() — aggregates              ║
// ║  9. T::q().order_by_desc().limit() — pagination             ║
// ║  10. T::delete() — simple delete                            ║
// ╚════════════════════════════════════════════════════════════╝
//
// Run:   cargo run -p vil-db-vilorm-crud
// Test:
//   # Create author
//   curl -X POST http://localhost:8080/api/blog/authors \
//     -H 'Content-Type: application/json' \
//     -d '{"name":"Alice","bio":"Rust enthusiast"}'
//
//   # Create post
//   curl -X POST http://localhost:8080/api/blog/posts \
//     -H 'Content-Type: application/json' \
//     -d '{"author_id":"<id>","title":"VilORM Guide","content":"..."}'
//
//   # List posts (slim projection)
//   curl http://localhost:8080/api/blog/posts
//
//   # Stats (aggregate)
//   curl http://localhost:8080/api/blog/stats

use serde::{Deserialize, Serialize};
use std::sync::Arc;
use vil_db_sqlx::SqlxPool;
use vil_orm::VilQuery;
use vil_orm_derive::VilEntity;
use vil_server::prelude::*;

// ── Models with VilEntity ──

#[derive(Debug, Clone, Serialize, Deserialize, VilModel, sqlx::FromRow, VilEntity)]
#[vil_entity(table = "authors")]
struct Author {
    #[vil_entity(pk)]
    id: String,
    name: String,
    bio: Option<String>,
    posts_count: i64,
    #[vil_entity(auto_now_add)]
    created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, VilModel, sqlx::FromRow, VilEntity)]
#[vil_entity(table = "posts")]
struct Post {
    #[vil_entity(pk)]
    id: String,
    author_id: String,
    title: String,
    content: String,
    status: String,
    views: i64,
    #[vil_entity(auto_now_add)]
    created_at: String,
    #[vil_entity(auto_now)]
    updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, VilModel, sqlx::FromRow, VilEntity)]
#[vil_entity(table = "tags")]
struct Tag {
    #[vil_entity(pk)]
    id: String,
    #[vil_entity(unique)]
    name: String,
}

// View types — slim projections (no VilEntity needed, just FromRow)
#[derive(Debug, Clone, Serialize, Deserialize, VilModel, sqlx::FromRow)]
struct PostListItem {
    id: String,
    title: String,
    author_name: String,
    status: String,
    views: i64,
    created_at: String,
}

// ── Request types ──

#[derive(Debug, Deserialize)]
struct CreateAuthor {
    name: String,
    bio: Option<String>,
}

#[derive(Debug, Deserialize)]
struct CreatePost {
    author_id: String,
    title: String,
    content: String,
    status: Option<String>,
}

#[derive(Debug, Deserialize)]
struct UpdatePost {
    title: Option<String>,
    content: Option<String>,
    status: Option<String>,
}

// ── State ──

#[derive(Clone)]
struct AppState {
    pool: Arc<SqlxPool>,
}

// ── Handlers ──

/// POST /authors — create author with optional bio (NULL-safe)
async fn create_author(ctx: ServiceCtx, body: ShmSlice) -> HandlerResult<VilResponse<Author>> {
    let state = ctx.state::<AppState>().expect("state");
    let req: CreateAuthor = body.json().map_err(|_| VilError::bad_request("invalid JSON"))?;
    let id = uuid::Uuid::new_v4().to_string();

    // Pattern: insert with Option<String> → NULL if None
    Author::q()
        .insert_columns(&["id", "name", "bio"])
        .value(id.clone())
        .value(req.name)
        .value_opt_str(req.bio)
        .execute(state.pool.inner())
        .await
        .map_err(|e| VilError::internal(format!("{e}")))?;

    let author = Author::find_by_id(state.pool.inner(), &id)
        .await
        .map_err(|e| VilError::internal(format!("{e}")))?
        .ok_or_else(|| VilError::internal("created but not found"))?;

    Ok(VilResponse::created(author))
}

/// GET /authors — list all authors
async fn list_authors(ctx: ServiceCtx) -> HandlerResult<VilResponse<Vec<Author>>> {
    let state = ctx.state::<AppState>().expect("state");
    let authors = Author::find_all(state.pool.inner())
        .await
        .map_err(|e| VilError::internal(format!("{e}")))?;
    Ok(VilResponse::ok(authors))
}

/// POST /posts — create post
async fn create_post(ctx: ServiceCtx, body: ShmSlice) -> HandlerResult<VilResponse<Post>> {
    let state = ctx.state::<AppState>().expect("state");
    let req: CreatePost = body.json().map_err(|_| VilError::bad_request("invalid JSON"))?;
    let id = uuid::Uuid::new_v4().to_string();
    let status = req.status.unwrap_or_else(|| "draft".to_string());

    Post::q()
        .insert_columns(&["id", "author_id", "title", "content", "status"])
        .value(id.clone())
        .value(req.author_id.clone())
        .value(req.title)
        .value(req.content)
        .value(status)
        .execute(state.pool.inner())
        .await
        .map_err(|e| VilError::internal(format!("{e}")))?;

    // Increment author posts_count
    Author::q()
        .update()
        .set_raw("posts_count", "posts_count + 1")
        .where_eq("id", &req.author_id)
        .execute(state.pool.inner())
        .await
        .map_err(|e| VilError::internal(format!("{e}")))?;

    let post = Post::find_by_id(state.pool.inner(), &id)
        .await
        .map_err(|e| VilError::internal(format!("{e}")))?
        .ok_or_else(|| VilError::internal("created but not found"))?;

    Ok(VilResponse::created(post))
}

/// GET /posts — list with JOIN (slim projection, not SELECT *)
async fn list_posts(ctx: ServiceCtx) -> HandlerResult<VilResponse<Vec<PostListItem>>> {
    let state = ctx.state::<AppState>().expect("state");

    // Pattern: JOIN + column projection + ordering
    let posts = Post::q()
        .select(&["p.id", "p.title", "a.name as author_name", "p.status", "p.views", "p.created_at"])
        .alias("p")
        .join("authors a", "a.id = p.author_id")
        .order_by_desc("p.created_at")
        .limit(50)
        .fetch_all::<PostListItem>(state.pool.inner())
        .await
        .map_err(|e| VilError::internal(format!("{e}")))?;

    Ok(VilResponse::ok(posts))
}

/// GET /posts/:id — full post by PK
async fn get_post(ctx: ServiceCtx, Path(id): Path<String>) -> HandlerResult<VilResponse<Post>> {
    let state = ctx.state::<AppState>().expect("state");

    // Pattern: increment views + fetch
    Post::q()
        .update()
        .set_raw("views", "views + 1")
        .where_eq("id", &id)
        .execute(state.pool.inner())
        .await
        .map_err(|e| VilError::internal(format!("{e}")))?;

    let post = Post::find_by_id(state.pool.inner(), &id)
        .await
        .map_err(|e| VilError::internal(format!("{e}")))?
        .ok_or_else(|| VilError::not_found("Post not found"))?;

    Ok(VilResponse::ok(post))
}

/// PUT /posts/:id — partial update (set_optional skips None fields)
async fn update_post(
    ctx: ServiceCtx,
    Path(id): Path<String>,
    body: ShmSlice,
) -> HandlerResult<VilResponse<Post>> {
    let state = ctx.state::<AppState>().expect("state");
    let req: UpdatePost = body.json().map_err(|_| VilError::bad_request("invalid JSON"))?;

    // Pattern: set_optional — only SET provided fields, skip None
    Post::q()
        .update()
        .set_optional("title", req.title.as_deref())
        .set_optional("content", req.content.as_deref())
        .set_optional("status", req.status.as_deref())
        .set_raw("updated_at", "datetime('now')")
        .where_eq("id", &id)
        .execute(state.pool.inner())
        .await
        .map_err(|e| VilError::internal(format!("{e}")))?;

    let post = Post::find_by_id(state.pool.inner(), &id)
        .await
        .map_err(|e| VilError::internal(format!("{e}")))?
        .ok_or_else(|| VilError::not_found("Post not found"))?;

    Ok(VilResponse::ok(post))
}

/// DELETE /posts/:id
async fn delete_post(ctx: ServiceCtx, Path(id): Path<String>) -> HandlerResult<VilResponse<serde_json::Value>> {
    let state = ctx.state::<AppState>().expect("state");
    let deleted = Post::delete(state.pool.inner(), &id)
        .await
        .map_err(|e| VilError::internal(format!("{e}")))?;

    if deleted {
        Ok(VilResponse::ok(serde_json::json!({"deleted": true, "id": id})))
    } else {
        Err(VilError::not_found("Post not found"))
    }
}

/// POST /tags — upsert tag (ON CONFLICT DO NOTHING)
async fn create_tag(ctx: ServiceCtx, body: ShmSlice) -> HandlerResult<VilResponse<serde_json::Value>> {
    let state = ctx.state::<AppState>().expect("state");
    let req: serde_json::Value = body.json().map_err(|_| VilError::bad_request("invalid JSON"))?;
    let name = req["name"].as_str().ok_or_else(|| VilError::bad_request("name required"))?;
    let id = uuid::Uuid::new_v4().to_string();

    // Pattern: ON CONFLICT DO NOTHING (idempotent upsert)
    Tag::q()
        .insert_columns(&["id", "name"])
        .value(id)
        .value(name.to_string())
        .on_conflict_nothing("name")
        .execute(state.pool.inner())
        .await
        .map_err(|e| VilError::internal(format!("{e}")))?;

    Ok(VilResponse::ok(serde_json::json!({"ok": true, "tag": name})))
}

/// GET /stats — aggregate queries via VilQuery scalar
async fn blog_stats(ctx: ServiceCtx) -> HandlerResult<VilResponse<serde_json::Value>> {
    let state = ctx.state::<AppState>().expect("state");
    let pool = state.pool.inner();

    let total_posts = Post::count(pool).await.unwrap_or(0);
    let total_authors = Author::count(pool).await.unwrap_or(0);

    // Pattern: scalar aggregate with condition
    let published: i64 = Post::q()
        .select_expr("CAST(COUNT(*) AS INTEGER)")
        .where_eq("status", "published")
        .scalar::<i64>(pool)
        .await
        .unwrap_or(0);

    let total_views: i64 = Post::q()
        .select_expr("COALESCE(CAST(SUM(views) AS INTEGER), 0)")
        .where_raw("1=1")
        .scalar::<i64>(pool)
        .await
        .unwrap_or(0);

    Ok(VilResponse::ok(serde_json::json!({
        "total_posts": total_posts,
        "published": published,
        "drafts": total_posts - published,
        "total_authors": total_authors,
        "total_views": total_views,
    })))
}

// ── Main ──

#[tokio::main]
async fn main() {
    let pool = SqlxPool::connect("blog", vil_db_sqlx::SqlxConfig::sqlite("sqlite:blog.db?mode=rwc"))
        .await
        .expect("SQLite connect failed");

    pool.execute_raw(
        "CREATE TABLE IF NOT EXISTS authors (
            id TEXT PRIMARY KEY, name TEXT NOT NULL, bio TEXT,
            posts_count INTEGER DEFAULT 0,
            created_at TEXT DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ','now'))
        );
        CREATE TABLE IF NOT EXISTS posts (
            id TEXT PRIMARY KEY, author_id TEXT NOT NULL REFERENCES authors(id),
            title TEXT NOT NULL, content TEXT NOT NULL,
            status TEXT DEFAULT 'draft', views INTEGER DEFAULT 0,
            created_at TEXT DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ','now')),
            updated_at TEXT DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ','now'))
        );
        CREATE TABLE IF NOT EXISTS tags (
            id TEXT PRIMARY KEY, name TEXT UNIQUE NOT NULL
        );"
    ).await.expect("Migration failed");

    let state = AppState { pool: Arc::new(pool) };

    let blog_svc = ServiceProcess::new("blog")
        .endpoint(Method::POST, "/authors", post(create_author))
        .endpoint(Method::GET, "/authors", get(list_authors))
        .endpoint(Method::GET, "/posts", get(list_posts))
        .endpoint(Method::POST, "/posts", post(create_post))
        .endpoint(Method::GET, "/posts/:id", get(get_post))
        .endpoint(Method::PUT, "/posts/:id", put(update_post))
        .endpoint(Method::DELETE, "/posts/:id", delete(delete_post))
        .endpoint(Method::POST, "/tags", post(create_tag))
        .endpoint(Method::GET, "/stats", get(blog_stats))
        .state(state);

    VilApp::new("vilorm-showcase")
        .port(8080)
        .observer(true)
        .service(blog_svc)
        .run()
        .await;
}
