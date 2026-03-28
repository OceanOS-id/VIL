# Database Connectors

VIL native database connectors (distinct from `integrations/database.md` which covers sqlx/sea-orm/redis). These connectors include automatic `#[connector_fault/event/state]` instrumentation.

## Quick Reference

| Connector | Crate | Category |
|-----------|-------|----------|
| MongoDB | vil_conn_mongo | Document |
| ClickHouse | vil_conn_clickhouse | Analytics |
| DynamoDB | vil_conn_dynamodb | Key-Value (AWS) |
| Cassandra | vil_conn_cassandra | Wide-column |
| Neo4j | vil_conn_neo4j | Graph |
| Elasticsearch | vil_conn_elastic | Search |
| InfluxDB | vil_conn_influxdb | Time-series |

Also see [integrations/database.md](../integrations/database.md) for SQL (PostgreSQL/MySQL/SQLite) and Redis.

## MongoDB (vil_conn_mongo)

```rust
use vil_conn_mongo::{MongoConnector, MongoConfig};
use serde::{Deserialize, Serialize};

let mongo = MongoConnector::new(MongoConfig {
    uri: "mongodb://localhost:27017".into(),
    database: "mydb".into(),
    ..Default::default()
}).await?;

// Insert
mongo.collection::<Order>("orders")
    .insert_one(&order, None).await?;

// Find
let orders: Vec<Order> = mongo.collection("orders")
    .find(doc! { "status": "pending" }, None)
    .await?.try_collect().await?;

// Update
mongo.collection::<Order>("orders")
    .update_one(
        doc! { "_id": &id },
        doc! { "$set": { "status": "shipped" } },
        None
    ).await?;
```

## ClickHouse (vil_conn_clickhouse)

```rust
use vil_conn_clickhouse::{ClickHouseConnector, ClickHouseConfig};

let ch = ClickHouseConnector::new(ClickHouseConfig {
    url: "http://localhost:8123".into(),
    database: "analytics".into(),
    user: "default".into(),
    password: "".into(),
    ..Default::default()
}).await?;

// Insert batch
ch.insert("events", &events).await?;

// Query
let rows: Vec<EventRow> = ch
    .query("SELECT * FROM events WHERE ts > ? LIMIT 1000")
    .bind(since_ts)
    .fetch_all::<EventRow>().await?;
```

Note: ClickHouse is also used as a vil_log drain — see [logging/drains.md](../logging/drains.md).

## DynamoDB (vil_conn_dynamodb)

```rust
use vil_conn_dynamodb::{DynamoConnector, DynamoConfig};

let dynamo = DynamoConnector::new(DynamoConfig {
    table: "orders".into(),
    region: "us-east-1".into(),
    ..Default::default()
}).await?;

// Put
dynamo.put_item(item! {
    "pk" => "order#123",
    "sk" => "v1",
    "status" => "pending",
}).await?;

// Get
let item = dynamo.get_item("order#123", "v1").await?;

// Query (GSI)
let items = dynamo.query()
    .index("StatusIndex")
    .key_condition("status = :s", attrs! { ":s" => "pending" })
    .send().await?;
```

## Cassandra (vil_conn_cassandra)

```rust
use vil_conn_cassandra::{CassandraConnector, CassandraConfig};

let cassandra = CassandraConnector::new(CassandraConfig {
    contact_points: vec!["127.0.0.1:9042".into()],
    keyspace: "mykeyspace".into(),
    ..Default::default()
}).await?;

// Execute CQL
cassandra.execute(
    "INSERT INTO events (id, ts, payload) VALUES (?, ?, ?)",
    (uuid, ts, payload)
).await?;

// Query
let rows = cassandra.query(
    "SELECT * FROM events WHERE id = ? LIMIT 100",
    (event_id,)
).await?;
```

## Neo4j (vil_conn_neo4j)

```rust
use vil_conn_neo4j::{Neo4jConnector, Neo4jConfig};

let neo4j = Neo4jConnector::new(Neo4jConfig {
    uri: "bolt://localhost:7687".into(),
    user: "neo4j".into(),
    password: "password".into(),
    ..Default::default()
}).await?;

// Cypher query
let result = neo4j.run(
    "MATCH (u:User)-[:FOLLOWS]->(f) WHERE u.id = $id RETURN f",
    params! { "id" => user_id }
).await?;

// Create relationship
neo4j.run(
    "MERGE (a:User {id: $a}) MERGE (b:User {id: $b}) MERGE (a)-[:FOLLOWS]->(b)",
    params! { "a" => user_a, "b" => user_b }
).await?;
```

## Elasticsearch (vil_conn_elastic)

```rust
use vil_conn_elastic::{ElasticConnector, ElasticConfig};

let elastic = ElasticConnector::new(ElasticConfig {
    url: "http://localhost:9200".into(),
    index: "products".into(),
    ..Default::default()
}).await?;

// Index document
elastic.index(&product).await?;

// Search
let results: Vec<Product> = elastic.search(json!({
    "query": {
        "multi_match": {
            "query": "wireless headphones",
            "fields": ["name", "description"]
        }
    }
})).fetch_all::<Product>().await?;

// Delete
elastic.delete(&product_id).await?;
```

## InfluxDB (vil_conn_influxdb)

```rust
use vil_conn_influxdb::{InfluxConnector, InfluxConfig, Point};

let influx = InfluxConnector::new(InfluxConfig {
    url: "http://localhost:8086".into(),
    token: std::env::var("INFLUX_TOKEN")?,
    org: "my-org".into(),
    bucket: "metrics".into(),
    ..Default::default()
}).await?;

// Write points
influx.write(vec![
    Point::measurement("cpu")
        .tag("host", "server1")
        .field("usage", 72.5_f64)
        .timestamp(Utc::now()),
]).await?;

// Query (Flux)
let series = influx.query(r#"
    from(bucket: "metrics")
        |> range(start: -1h)
        |> filter(fn: (r) => r._measurement == "cpu")
"#).await?;
```

## VilApp Integration

```rust
let service = ServiceProcess::new("data")
    .extension(mongo.clone())
    .extension(elastic.clone())
    .endpoint(Method::POST, "/ingest", post(ingest));

#[vil_handler(shm)]
async fn ingest(ctx: ServiceCtx, slice: ShmSlice) -> VilResponse<String> {
    let record: Record = slice.json()?;
    let mongo = ctx.state::<MongoConnector>();
    let elastic = ctx.state::<ElasticConnector>();

    let id = mongo.collection("records").insert_one(&record, None).await?.inserted_id;
    elastic.index(&record).await?;
    VilResponse::ok(id.to_string())
}
```

## Events & Faults

All connectors emit via `#[connector_event]` and `#[connector_fault]` automatically.
See [macros.md](macros.md) for details.
