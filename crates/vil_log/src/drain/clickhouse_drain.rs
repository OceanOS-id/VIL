// =============================================================================
// vil_log::drain::clickhouse_drain — ClickHouse Batch Drain
// =============================================================================
//
// Writes log events to ClickHouse via batch INSERT using RowBinary format.
//
// Architecture:
//   flush() receives &[LogSlot] → converts to LogRow → INSERT into vil_log table
//
// Table: single `vil_log` table with all categories (v0.1).
//        Per-category tables planned for v0.2.
//
// Requires feature: clickhouse-drain
// =============================================================================

use async_trait::async_trait;
use serde::Serialize;

use crate::drain::traits::LogDrain;
use crate::types::LogSlot;

/// ClickHouse drain configuration.
#[derive(Debug, Clone)]
pub struct ClickHouseConfig {
    /// ClickHouse HTTP URL (e.g., "http://localhost:8123")
    pub url: String,
    /// Database name (e.g., "vil_logs")
    pub database: String,
    /// Table name (default: "vil_log")
    pub table: String,
}

impl Default for ClickHouseConfig {
    fn default() -> Self {
        Self {
            url: "http://localhost:8123".to_string(),
            database: "default".to_string(),
            table: "vil_log".to_string(),
        }
    }
}

/// Row struct for ClickHouse RowBinary INSERT.
#[derive(Debug, clickhouse::Row, Serialize)]
struct LogRow {
    event_id_high: u64,
    event_id_low: u64,
    trace_id: u64,
    tenant_id: u64,
    process_id: u64,
    timestamp_ns: u64,
    level: u8,
    category: u8,
    subcategory: u8,
    version: u8,
    service_hash: u32,
    handler_hash: u32,
    node_hash: u32,
    payload: Vec<u8>,
}

impl LogRow {
    fn from_slot(slot: &LogSlot) -> Self {
        let h = &slot.header;
        Self {
            event_id_high: (h.event_id >> 64) as u64,
            event_id_low: h.event_id as u64,
            trace_id: h.trace_id,
            tenant_id: h.tenant_id,
            process_id: h.process_id,
            timestamp_ns: h.timestamp_ns,
            level: h.level,
            category: h.category,
            subcategory: h.subcategory,
            version: h.version,
            service_hash: h.service_hash,
            handler_hash: h.handler_hash,
            node_hash: h.node_hash,
            payload: slot.payload.to_vec(),
        }
    }
}

/// ClickHouse batch drain.
///
/// Inserts log events into a single ClickHouse table using RowBinary format.
/// Batching is handled by the VIL log runtime — this drain receives pre-batched slices.
pub struct ClickHouseDrain {
    client: clickhouse::Client,
    table: String,
}

impl ClickHouseDrain {
    /// Create a new ClickHouse drain from config.
    pub fn new(config: ClickHouseConfig) -> Self {
        let client = clickhouse::Client::default()
            .with_url(&config.url)
            .with_database(&config.database);
        Self {
            client,
            table: config.table,
        }
    }

    /// Create table DDL. Call this once at startup if needed.
    pub async fn create_table(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let ddl = format!(
            r#"
            CREATE TABLE IF NOT EXISTS {} (
                event_id_high UInt64,
                event_id_low  UInt64,
                trace_id      UInt64,
                tenant_id     UInt64,
                process_id    UInt64,
                timestamp_ns  UInt64,
                level         UInt8,
                category      UInt8,
                subcategory   UInt8,
                version       UInt8,
                service_hash  UInt32,
                handler_hash  UInt32,
                node_hash     UInt32,
                payload       String
            ) ENGINE = MergeTree()
            PARTITION BY toDate(fromUnixTimestamp64Nano(timestamp_ns))
            ORDER BY (category, level, timestamp_ns)
            TTL toDate(fromUnixTimestamp64Nano(timestamp_ns)) + INTERVAL 90 DAY
            SETTINGS index_granularity = 8192
            "#,
            self.table
        );
        self.client.query(&ddl).execute().await?;
        Ok(())
    }
}

#[async_trait]
impl LogDrain for ClickHouseDrain {
    fn name(&self) -> &'static str {
        "clickhouse"
    }

    async fn flush(&mut self, batch: &[LogSlot]) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        if batch.is_empty() {
            return Ok(());
        }

        let mut insert = self.client.insert(&self.table)?;
        for slot in batch {
            insert.write(&LogRow::from_slot(slot)).await?;
        }
        insert.end().await?;
        Ok(())
    }

    async fn shutdown(&mut self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        Ok(())
    }
}
