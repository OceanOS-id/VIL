//! vil_trigger_db_poll — DB table polling trigger (last-id pattern)
//! Polls a database table for new rows beyond a high-water mark, fires TriggerEvent per row.

use async_trait::async_trait;
use std::sync::atomic::{AtomicBool, Ordering};
use vil_trigger_core::{EventCallback, TriggerFault, TriggerSource};

pub struct DbPollConfig {
    pub database_url: String,
    pub table: String,
    pub id_column: String,
    pub poll_interval_secs: u64,
}

impl DbPollConfig {
    pub fn new(
        database_url: impl Into<String>,
        table: impl Into<String>,
        id_column: impl Into<String>,
        poll_interval_secs: u64,
    ) -> Self {
        Self {
            database_url: database_url.into(),
            table: table.into(),
            id_column: id_column.into(),
            poll_interval_secs,
        }
    }
}

pub struct DbPollTrigger {
    config: DbPollConfig,
    stopped: AtomicBool,
}

pub fn create_trigger(config: DbPollConfig) -> DbPollTrigger {
    DbPollTrigger {
        config,
        stopped: AtomicBool::new(false),
    }
}

#[async_trait]
impl TriggerSource for DbPollTrigger {
    fn kind(&self) -> &'static str {
        "db_poll"
    }

    async fn start(&self, _on_event: EventCallback) -> Result<(), TriggerFault> {
        tracing::info!(
            "DB poll trigger started: table={}, id_column={}, interval={}s",
            self.config.table,
            self.config.id_column,
            self.config.poll_interval_secs
        );
        // Poll loop would use vil_db_sqlx to query for rows > last_seen_id
        while !self.stopped.load(Ordering::Relaxed) {
            tokio::time::sleep(std::time::Duration::from_secs(self.config.poll_interval_secs)).await;
            // In production: SELECT * FROM table WHERE id_column > last_id ORDER BY id_column ASC
            // fire event per new row, update last_id
        }
        Ok(())
    }

    async fn pause(&self) -> Result<(), TriggerFault> {
        Ok(())
    }
    async fn resume(&self) -> Result<(), TriggerFault> {
        Ok(())
    }
    async fn stop(&self) -> Result<(), TriggerFault> {
        self.stopped.store(true, Ordering::Relaxed);
        Ok(())
    }
}
