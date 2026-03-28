// =============================================================================
// vil_log::drain::fallback — FallbackDrain
// =============================================================================
//
// Wraps any LogDrain with a file-based fallback. When the primary drain fails
// consecutively (exceeding a configurable threshold), log events are written
// to a fallback JSONL file instead of being dropped.
//
// When the primary drain recovers, the fallback file remains for manual replay.
// =============================================================================

use std::fs::{File, OpenOptions};
use std::io::{BufWriter, Write};
use std::path::PathBuf;

use async_trait::async_trait;

use crate::drain::traits::LogDrain;
use crate::types::{LogCategory, LogLevel, LogSlot};

/// Wraps a primary drain with a file-based fallback for resilience.
///
/// When the primary drain fails `max_failures_before_fallback` times
/// consecutively, subsequent batches are written to a JSONL fallback file
/// instead of being silently dropped.
pub struct FallbackDrain<D: LogDrain> {
    primary: D,
    fallback_path: PathBuf,
    fallback_writer: Option<BufWriter<File>>,
    consecutive_failures: u32,
    max_failures_before_fallback: u32,
}

impl<D: LogDrain> FallbackDrain<D> {
    /// Create a new FallbackDrain wrapping the given primary drain.
    ///
    /// - `primary` — the main drain to try first
    /// - `fallback_path` — path to the JSONL fallback file
    /// - `max_failures` — number of consecutive failures before switching to fallback
    pub fn new(primary: D, fallback_path: PathBuf, max_failures: u32) -> Self {
        Self {
            primary,
            fallback_path,
            fallback_writer: None,
            consecutive_failures: 0,
            max_failures_before_fallback: max_failures.max(1),
        }
    }

    /// Write a batch to the fallback file.
    fn write_to_fallback(
        &mut self,
        batch: &[LogSlot],
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // Lazily open the fallback writer
        if self.fallback_writer.is_none() {
            let file = OpenOptions::new()
                .create(true)
                .append(true)
                .open(&self.fallback_path)?;
            self.fallback_writer = Some(BufWriter::new(file));
        }

        if let Some(writer) = self.fallback_writer.as_mut() {
            for slot in batch {
                let level = LogLevel::from(slot.header.level);
                let category = LogCategory::from(slot.header.category);

                let record = serde_json::json!({
                    "ts":       slot.header.timestamp_ns,
                    "level":    level.to_string(),
                    "category": category.to_string(),
                    "svc":      slot.header.service_hash,
                    "handler":  slot.header.handler_hash,
                    "pid":      slot.header.process_id,
                    "trace_id": slot.header.trace_id,
                    "fallback": true,
                });

                let line = serde_json::to_string(&record)?;
                writeln!(writer, "{}", line)?;
            }
            writer.flush()?;
        }
        Ok(())
    }
}

#[async_trait]
impl<D: LogDrain> LogDrain for FallbackDrain<D> {
    fn name(&self) -> &'static str {
        "fallback"
    }

    async fn flush(
        &mut self,
        batch: &[LogSlot],
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        match self.primary.flush(batch).await {
            Ok(()) => {
                self.consecutive_failures = 0;
                Ok(())
            }
            Err(e) => {
                self.consecutive_failures += 1;
                if self.consecutive_failures >= self.max_failures_before_fallback {
                    // Write to fallback file instead of dropping
                    if let Err(fallback_err) = self.write_to_fallback(batch) {
                        eprintln!(
                            "[vil_log] Primary drain '{}' failed: {}. Fallback also failed: {}",
                            self.primary.name(),
                            e,
                            fallback_err
                        );
                    } else {
                        eprintln!(
                            "[vil_log] Primary drain '{}' failed ({} consecutive). \
                             Wrote {} events to fallback {:?}",
                            self.primary.name(),
                            self.consecutive_failures,
                            batch.len(),
                            self.fallback_path
                        );
                    }
                }
                Err(e)
            }
        }
    }

    async fn shutdown(&mut self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // Flush fallback writer if open
        if let Some(mut w) = self.fallback_writer.take() {
            let _ = w.flush();
        }
        self.primary.shutdown().await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::drain::null::NullDrain;

    #[tokio::test]
    async fn test_fallback_drain_success() {
        let primary = NullDrain;
        let path = std::env::temp_dir().join("vil_fallback_test_success.jsonl");
        let mut drain = FallbackDrain::new(primary, path.clone(), 3);

        let batch = vec![LogSlot::default()];
        let result = drain.flush(&batch).await;
        assert!(result.is_ok());
        assert_eq!(drain.consecutive_failures, 0);

        let _ = std::fs::remove_file(&path);
    }

    #[tokio::test]
    async fn test_fallback_drain_name() {
        let primary = NullDrain;
        let path = std::env::temp_dir().join("vil_fallback_test_name.jsonl");
        let drain = FallbackDrain::new(primary, path, 3);
        assert_eq!(drain.name(), "fallback");
    }
}
