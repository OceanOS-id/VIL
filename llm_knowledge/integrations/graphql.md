# GraphQL Integration

`vil_graphql` provides resolver-based GraphQL with subscriptions and playground.

## Setup

```rust
use vil_graphql::prelude::*;

let schema = VilSchema::build(QueryRoot, MutationRoot, SubscriptionRoot)
    .data(db_pool.clone())
    .finish();

let service = ServiceProcess::new("graphql")
    .extension(schema)
    .endpoint(Method::POST, "/graphql", post(graphql_handler))
    .endpoint(Method::GET, "/graphql/playground", get(playground_handler));

VilApp::new("graphql-api")
    .port(8080)
    .service(service)
    .run()
    .await;
```

## Query Resolver

```rust
struct QueryRoot;

#[VilObject]
impl QueryRoot {
    async fn user(&self, ctx: &VilContext<'_>, id: i64) -> Result<User> {
        let db = ctx.data::<VilDbPool>()?;
        let user = sqlx::query_as::<_, User>("SELECT * FROM users WHERE id = $1")
            .bind(id)
            .fetch_one(db.as_ref())
            .await?;
        Ok(user)
    }

    async fn users(&self, ctx: &VilContext<'_>, limit: Option<i64>) -> Result<Vec<User>> {
        let db = ctx.data::<VilDbPool>()?;
        let limit = limit.unwrap_or(20);
        let users = sqlx::query_as::<_, User>("SELECT * FROM users LIMIT $1")
            .bind(limit)
            .fetch_all(db.as_ref())
            .await?;
        Ok(users)
    }
}
```

## Mutation Resolver

```rust
struct MutationRoot;

#[VilObject]
impl MutationRoot {
    async fn create_user(&self, ctx: &VilContext<'_>, input: CreateUserInput) -> Result<User> {
        let db = ctx.data::<VilDbPool>()?;
        let user = insert_user(db, &input).await?;
        Ok(user)
    }
}
```

## Subscriptions

```rust
struct SubscriptionRoot;

#[VilSubscription]
impl SubscriptionRoot {
    async fn order_updates(&self, ctx: &VilContext<'_>) -> impl Stream<Item = OrderUpdate> {
        let hub = ctx.data::<SseHub>()?;
        hub.subscribe("order_updates")
    }
}
```

## Playground

Built-in GraphQL Playground at `/graphql/playground`:

```rust
async fn playground_handler() -> impl IntoResponse {
    VilPlayground::new("/graphql").into_response()
}
```

## Pagination

```rust
async fn users_paginated(&self, ctx: &VilContext<'_>, after: Option<String>, first: i64) -> Result<Connection<User>> {
    let db = ctx.data::<VilDbPool>()?;
    paginate(db, after, first).await
}
```

> Reference: docs/vil/006-VIL-Developer_Guide-CLI-Deployment.md
