// 605 — Blog Platform (Mixed: VilQuery YAML + NativeCode with VilQuery builder)
//
// VilQuery inline YAML (5): GET /authors, GET /posts (JOIN), GET /posts/:id,
//   GET /stats (COUNT), DELETE /posts/:id
// NativeCode with VilQuery builder (4): POST /authors, POST /posts,
//   PUT /posts/:id, POST /tags
use serde_json::json;
use std::sync::{Arc, OnceLock};
use vil_orm::VilQuery;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, sqlx::FromRow)]
struct Author {
    id: String,
    name: String,
    bio: Option<String>,
    created_at: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, sqlx::FromRow)]
struct Post {
    id: String,
    author_id: String,
    title: String,
    content: String,
    status: String,
    views: i64,
    created_at: String,
}

fn uuid() -> String {
    use std::sync::atomic::{AtomicU64, Ordering};
    static C: AtomicU64 = AtomicU64::new(1);
    let t = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos();
    let c = C.fetch_add(1, Ordering::Relaxed);
    format!("{:08x}-{:04x}-4{:03x}-{:04x}-{:012x}",
        (t >> 32) as u32, (t >> 16) as u16 & 0xffff, t as u16 & 0x0fff,
        0x8000 | (c as u16 & 0x3fff), t as u64 & 0xffffffffffff)
}

static POOL: OnceLock<Arc<vil_db_sqlx::SqlxPool>> = OnceLock::new();
fn pool() -> &'static Arc<vil_db_sqlx::SqlxPool> { POOL.get().expect("pool") }

fn block_async<F: std::future::Future>(f: F) -> F::Output {
    tokio::task::block_in_place(|| tokio::runtime::Handle::current().block_on(f))
}

#[tokio::main]
async fn main() {
    if std::env::var("VIL_DATABASE_URL").is_err() {
        let p = format!("{}/blog_vwfd.db", std::env::temp_dir().display());
        std::env::set_var("VIL_DATABASE_URL", format!("sqlite:{}?mode=rwc", p));
    }
    let url = std::env::var("VIL_DATABASE_URL").unwrap();
    let db = vil_db_sqlx::SqlxPool::connect("blog", vil_db_sqlx::SqlxConfig::sqlite(&url))
        .await.expect("db");

    // Init tables
    db.execute_raw("CREATE TABLE IF NOT EXISTS authors (id TEXT PRIMARY KEY, name TEXT NOT NULL, bio TEXT, created_at TEXT DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ','now')))").await.ok();
    db.execute_raw("CREATE TABLE IF NOT EXISTS posts (id TEXT PRIMARY KEY, author_id TEXT, title TEXT NOT NULL, content TEXT DEFAULT '', status TEXT DEFAULT 'draft', views INTEGER DEFAULT 0, created_at TEXT DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ','now')))").await.ok();
    db.execute_raw("CREATE TABLE IF NOT EXISTS tags (id INTEGER PRIMARY KEY AUTOINCREMENT, name TEXT UNIQUE NOT NULL)").await.ok();

    POOL.set(Arc::new(db)).unwrap_or_else(|_| panic!("pool"));

    vil_vwfd::app("examples/605-db-vilorm-crud/vwfd/workflows", 8080)
        // ── NativeCode with VilQuery builder ──

        .native("create_author", |input| {
            let body = &input["body"];
            let id = uuid();
            let name = body["name"].as_str().unwrap_or("Unknown").to_string();
            let bio = body["bio"].as_str().map(|s| s.to_string());

            block_async(async {
                let mut q = VilQuery::new("authors")
                    .insert_columns(&["id", "name"])
                    .value(id.clone())
                    .value(name.clone());
                if let Some(ref b) = bio {
                    q = VilQuery::new("authors")
                        .insert_columns(&["id", "name", "bio"])
                        .value(id.clone())
                        .value(name.clone())
                        .value(b.clone());
                }
                q.execute(pool().inner()).await.ok();

                let author = VilQuery::new("authors")
                    .where_eq("id", &id)
                    .fetch_one::<Author>(pool().inner())
                    .await
                    .map_err(|e| format!("500:{}", e))?;
                Ok(json!({"_status": 201, "id": author.id, "name": author.name, "bio": author.bio}))
            })
        })

        .native("create_post", |input| {
            let body = &input["body"];
            let id = uuid();
            let author_id = body["author_id"].as_str().unwrap_or("").to_string();
            let title = body["title"].as_str().unwrap_or("Untitled").to_string();
            let content = body["content"].as_str().unwrap_or("").to_string();
            let status = body["status"].as_str().unwrap_or("draft").to_string();

            block_async(async {
                VilQuery::new("posts")
                    .insert_columns(&["id", "author_id", "title", "content", "status"])
                    .value(id.clone())
                    .value(author_id)
                    .value(title)
                    .value(content)
                    .value(status)
                    .execute(pool().inner())
                    .await
                    .map_err(|e| format!("500:{}", e))?;

                let post = VilQuery::new("posts")
                    .where_eq("id", &id)
                    .fetch_one::<Post>(pool().inner())
                    .await
                    .map_err(|e| format!("500:{}", e))?;
                Ok(json!({"id": post.id, "title": post.title, "status": post.status, "views": post.views}))
            })
        })

        .native("update_post", |input| {
            let body = &input["body"];
            let path = input["path"].as_str().unwrap_or("");
            let id = path.split('/').last().unwrap_or("").to_string();

            block_async(async {
                let q = VilQuery::new("posts")
                    .update()
                    .set_optional("title", body["title"].as_str())
                    .set_optional("content", body["content"].as_str())
                    .set_optional("status", body["status"].as_str())
                    .where_eq("id", &id);
                q.execute(pool().inner()).await.map_err(|e| format!("500:{}", e))?;

                let post = VilQuery::new("posts")
                    .where_eq("id", &id)
                    .fetch_optional::<Post>(pool().inner())
                    .await
                    .map_err(|e| format!("500:{}", e))?;
                match post {
                    Some(p) => Ok(json!({"id": p.id, "title": p.title, "updated": true})),
                    None => Err("404:post not found".into()),
                }
            })
        })

        .native("create_tag", |input| {
            let body = &input["body"];
            let name = body["name"].as_str().unwrap_or("tag").to_string();

            block_async(async {
                VilQuery::new("tags")
                    .insert_columns(&["name"])
                    .value(name.clone())
                    .on_conflict_nothing("name")
                    .execute(pool().inner())
                    .await
                    .map_err(|e| format!("500:{}", e))?;
                Ok(json!({"name": name, "created": true}))
            })
        })

        .run()
        .await;
}
