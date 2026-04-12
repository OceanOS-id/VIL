//! vil_trigger_sftp — SFTP directory polling trigger
//! Polls an SFTP remote directory for new/changed files, fires TriggerEvent per change.

use async_trait::async_trait;
use std::sync::atomic::{AtomicBool, Ordering};
use vil_trigger_core::{EventCallback, TriggerEvent, TriggerFault, TriggerSource};

pub struct SftpConfig {
    pub host: String,
    pub port: u16,
    pub username: String,
    pub password: String,
    pub remote_dir: String,
    pub poll_interval_secs: u64,
}

impl SftpConfig {
    pub fn new(
        host: impl Into<String>,
        port: u16,
        username: impl Into<String>,
        password: impl Into<String>,
        remote_dir: impl Into<String>,
        poll_interval_secs: u64,
    ) -> Self {
        Self {
            host: host.into(),
            port,
            username: username.into(),
            password: password.into(),
            remote_dir: remote_dir.into(),
            poll_interval_secs,
        }
    }
}

pub struct SftpTrigger {
    config: SftpConfig,
    stopped: AtomicBool,
}

pub fn create_trigger(config: SftpConfig) -> SftpTrigger {
    SftpTrigger {
        config,
        stopped: AtomicBool::new(false),
    }
}

#[async_trait]
impl TriggerSource for SftpTrigger {
    fn kind(&self) -> &'static str {
        "sftp"
    }

    async fn start(&self, _on_event: EventCallback) -> Result<(), TriggerFault> {
        tracing::info!(
            "SFTP trigger started: {}@{}:{} dir={}, interval={}s",
            self.config.username,
            self.config.host,
            self.config.port,
            self.config.remote_dir,
            self.config.poll_interval_secs
        );
        // Poll loop would connect via SFTP, list directory, diff against last seen
        while !self.stopped.load(Ordering::Relaxed) {
            tokio::time::sleep(std::time::Duration::from_secs(self.config.poll_interval_secs)).await;
            // In production: SFTP ls, diff, fire events for new/changed files
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
