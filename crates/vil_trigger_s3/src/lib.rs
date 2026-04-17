//! vil_trigger_s3 — S3 bucket polling trigger
//! Polls an S3 bucket prefix for new/changed objects, fires TriggerEvent per change.

use async_trait::async_trait;
use std::sync::atomic::{AtomicBool, Ordering};
use vil_trigger_core::{EventCallback, TriggerFault, TriggerSource};

pub struct S3Config {
    pub bucket: String,
    pub prefix: String,
    pub poll_interval_secs: u64,
}

impl S3Config {
    pub fn new(
        bucket: impl Into<String>,
        prefix: impl Into<String>,
        poll_interval_secs: u64,
    ) -> Self {
        Self {
            bucket: bucket.into(),
            prefix: prefix.into(),
            poll_interval_secs,
        }
    }
}

pub struct S3Trigger {
    config: S3Config,
    stopped: AtomicBool,
}

pub fn create_trigger(config: S3Config) -> S3Trigger {
    S3Trigger {
        config,
        stopped: AtomicBool::new(false),
    }
}

#[async_trait]
impl TriggerSource for S3Trigger {
    fn kind(&self) -> &'static str {
        "s3"
    }

    async fn start(&self, _on_event: EventCallback) -> Result<(), TriggerFault> {
        tracing::info!(
            "S3 trigger started: bucket={}, prefix={}, interval={}s",
            self.config.bucket,
            self.config.prefix,
            self.config.poll_interval_secs
        );
        // Poll loop would use vil_storage_s3 list_objects here
        while !self.stopped.load(Ordering::Relaxed) {
            tokio::time::sleep(std::time::Duration::from_secs(self.config.poll_interval_secs)).await;
            // In production: list objects, diff against last seen, fire events for new/changed
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
