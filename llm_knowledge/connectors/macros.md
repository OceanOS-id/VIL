# Connector Macros

VIL provides three attribute macros for instrumenting custom connectors: `#[connector_fault]`, `#[connector_event]`, and `#[connector_state]`.

## Quick Reference

| Macro | Purpose | vil_log type |
|-------|---------|-------------|
| `#[connector_fault]` | Emit on error return | system_log!(Error, ...) |
| `#[connector_event]` | Emit on success | system_log!(Info, ...) |
| `#[connector_state]` | Track state changes | system_log!(Debug, ...) |

All built-in connectors (vil_conn_*) apply these automatically. Use them when building custom connectors or wrapping third-party clients.

## #[connector_fault]

Emits a `system_log!` with the error details when the function returns `Err(...)`.

```rust
use vil_conn::prelude::*;

struct MyConnector {
    client: reqwest::Client,
    base_url: String,
}

impl MyConnector {
    #[connector_fault(name = "my_connector.fetch")]
    async fn fetch(&self, id: u64) -> Result<Record, ConnectorError> {
        let resp = self.client
            .get(format!("{}/records/{}", self.base_url, id))
            .send().await
            .map_err(ConnectorError::from)?;
        resp.json::<Record>().await.map_err(ConnectorError::from)
    }
}
```

Generated log entry on error:
```json
{
  "type": "system_log",
  "level": "Error",
  "connector": "my_connector.fetch",
  "error": "connection refused",
  "kind": "fault"
}
```

## #[connector_event]

Emits a `system_log!` on successful function return.

```rust
impl MyConnector {
    #[connector_event(name = "my_connector.send", fields(id, bytes_len))]
    async fn send(&self, id: u64, payload: &[u8]) -> Result<(), ConnectorError> {
        self.client
            .post(format!("{}/records/{}", self.base_url, id))
            .body(payload.to_vec())
            .send().await?;
        Ok(())
    }
}
```

Generated log entry on success:
```json
{
  "type": "system_log",
  "level": "Info",
  "connector": "my_connector.send",
  "id": 123,
  "bytes_len": 256,
  "kind": "event"
}
```

## #[connector_state]

Emits when connector internal state changes (connection open/close, pool resize, etc).

```rust
impl MyConnector {
    #[connector_state(name = "my_connector.connected")]
    async fn connect(&mut self) -> Result<(), ConnectorError> {
        self.connection = Some(establish_connection(&self.base_url).await?);
        Ok(())
    }

    #[connector_state(name = "my_connector.disconnected")]
    async fn disconnect(&mut self) -> Result<(), ConnectorError> {
        if let Some(conn) = self.connection.take() {
            conn.close().await?;
        }
        Ok(())
    }
}
```

## Combining All Three

Typical pattern for a production connector:

```rust
impl DatabaseConnector {
    #[connector_state(name = "db.pool_ready")]
    pub async fn init(&mut self) -> Result<(), ConnectorError> {
        self.pool = Some(Pool::connect(&self.url).await?);
        Ok(())
    }

    #[connector_event(name = "db.query", fields(query, duration_us))]
    pub async fn execute(&self, query: &str) -> Result<Vec<Row>, ConnectorError> {
        let start = Instant::now();
        let rows = self.pool().execute(query).await?;
        let _duration_us = start.elapsed().as_micros();
        Ok(rows)
    }

    #[connector_fault(name = "db.query_failed")]
    pub async fn execute_critical(&self, query: &str) -> Result<Vec<Row>, ConnectorError> {
        self.pool().execute(query).await.map_err(ConnectorError::from)
    }
}
```

## API

### Macro Parameters

| Parameter | Applies to | Description |
|-----------|-----------|-------------|
| `name` | all | Log entry connector name (required) |
| `fields(a, b, ...)` | connector_event | Capture fn args/locals as log fields |
| `level` | all | Override log level (default: Info/Error/Debug) |

### ConnectorError

All connectors use `vil_conn::ConnectorError` as the error type:

```rust
use vil_conn::ConnectorError;

#[derive(thiserror::Error, Debug)]
pub enum MyError {
    #[error("network: {0}")]
    Network(#[from] reqwest::Error),
}

impl From<MyError> for ConnectorError {
    fn from(e: MyError) -> Self {
        ConnectorError::new(e.to_string())
    }
}
```

## Common Patterns

### Wrap third-party client
```rust
pub struct S3Wrapper(aws_sdk_s3::Client);

impl S3Wrapper {
    #[connector_event(name = "s3.put", fields(key))]
    pub async fn put(&self, key: &str, data: Bytes) -> Result<(), ConnectorError> {
        self.0.put_object()
            .bucket(&self.bucket)
            .key(key)
            .body(data.into())
            .send().await
            .map_err(|e| ConnectorError::new(e.to_string()))?;
        Ok(())
    }

    #[connector_fault(name = "s3.get_failed")]
    pub async fn get(&self, key: &str) -> Result<Bytes, ConnectorError> {
        let out = self.0.get_object()
            .bucket(&self.bucket)
            .key(key)
            .send().await
            .map_err(|e| ConnectorError::new(e.to_string()))?;
        Ok(out.body.collect().await?.into_bytes())
    }
}
```
