# Database Integrations

VIL supports three database crates: `vil_db_sqlx` (raw SQL), `vil_db_sea_orm` (ORM), and `vil_db_redis` (cache/KV).

## vil_db_sqlx

Direct SQL with connection pooling, multi-database support.

```rust
use vil_db_sqlx::prelude::*;

let pool = VilDbPool::new()
    .url("postgres://user:pass@localhost:5432/mydb")
    .max_connections(20)
    .build()
    .await?;

// Register as state
let service = ServiceProcess::new("api")
    .extension(pool.clone())
    .endpoint(Method::GET, "/users/:id", get(get_user));
```

### Query in Handler

```rust
#[vil_handler(shm)]
async fn get_user(ctx: ServiceCtx, Path(id): Path<i64>) -> VilResponse<User> {
    let pool = ctx.state::<VilDbPool>();
    let user = sqlx::query_as::<_, User>("SELECT * FROM users WHERE id = $1")
        .bind(id)
        .fetch_one(pool.as_ref())
        .await
        .map_err(|_| VilError::not_found("User not found"))?;
    VilResponse::ok(user)
}
```

### Supported Databases

| Database | URL Scheme | Feature Flag |
|----------|-----------|--------------|
| PostgreSQL | `postgres://` | `postgres` |
| MySQL | `mysql://` | `mysql` |
| SQLite | `sqlite://` | `sqlite` |

## vil_db_sea_orm

ORM with entity generation, migrations, and typed queries.

```rust
use vil_db_sea_orm::prelude::*;

let db = VilOrmPool::new()
    .url("postgres://user:pass@localhost:5432/mydb")
    .build()
    .await?;

// Entity query
let users = UserEntity::find()
    .filter(user::Column::Active.eq(true))
    .all(db.as_ref())
    .await?;

// Insert
let new_user = user::ActiveModel {
    name: Set("Alice".to_string()),
    email: Set("alice@example.com".to_string()),
    ..Default::default()
};
let result = UserEntity::insert(new_user).exec(db.as_ref()).await?;
```

## vil_db_redis

Redis connection pool with caching helpers.

```rust
use vil_db_redis::prelude::*;

let redis = VilRedisPool::new()
    .url("redis://localhost:6379")
    .build()
    .await?;

// Set/Get
redis.set("key", "value", Some(Duration::from_secs(300))).await?;
let val: Option<String> = redis.get("key").await?;

// In handler
#[vil_handler(shm)]
async fn cached_user(ctx: ServiceCtx, Path(id): Path<i64>) -> VilResponse<User> {
    let redis = ctx.state::<VilRedisPool>();
    let cache_key = format!("user:{}", id);

    if let Some(cached) = redis.get::<User>(&cache_key).await? {
        return VilResponse::ok(cached);
    }

    let db = ctx.state::<VilDbPool>();
    let user = fetch_user(db, id).await?;
    redis.set(&cache_key, &user, Some(Duration::from_secs(60))).await?;
    VilResponse::ok(user)
}
```

## Additional Database Connectors

For databases beyond PostgreSQL/MySQL/SQLite/Redis, see the native connector crates:

| Database | Connector |
|----------|-----------|
| MongoDB | vil_conn_mongo |
| ClickHouse | vil_conn_clickhouse |
| DynamoDB | vil_conn_dynamodb |
| Cassandra | vil_conn_cassandra |
| Neo4j | vil_conn_neo4j |
| Elasticsearch | vil_conn_elastic |
| InfluxDB (time-series) | vil_conn_influxdb |

Full reference: [connectors/databases.md](../connectors/databases.md)

> Reference: docs/vil/006-VIL-Developer_Guide-CLI-Deployment.md
