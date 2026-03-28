// =============================================================================
// vil_log::drain::file — FileDrain with rolling rotation
// =============================================================================
//
// Writes JSON Lines to a rotating log file.
// Rotation strategies:
//   Daily  — rotate at midnight UTC
//   Hourly — rotate each hour
//   Size   — rotate when file exceeds max_bytes
//
// On rotation, the old file is renamed with a timestamp suffix.
// Optionally retains only the last `max_files` files.
// No gzip by default (v0.1 — keep it simple).
// =============================================================================

use std::fs::{self, File, OpenOptions};
use std::io::{BufWriter, Write};
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use async_trait::async_trait;

use crate::drain::traits::LogDrain;
use crate::types::{LogCategory, LogLevel, LogSlot};

/// File rotation strategy.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RotationStrategy {
    /// Rotate once per calendar day (UTC).
    Daily,
    /// Rotate once per hour (UTC).
    Hourly,
    /// Rotate when current file exceeds `max_bytes` bytes.
    Size { max_bytes: u64 },
}

/// Drain that writes JSON Lines to a rolling file.
pub struct FileDrain {
    dir: PathBuf,
    prefix: String,
    rotation: RotationStrategy,
    max_files: usize,
    writer: Option<BufWriter<File>>,
    current_bytes: u64,
    current_slot: u64, // hour-or-day slot for time-based rotation
}

impl FileDrain {
    /// Create a new `FileDrain`.
    ///
    /// - `dir`      — directory for log files
    /// - `prefix`   — filename prefix (e.g. "app")
    /// - `rotation` — rotation strategy
    /// - `max_files`— how many rotated files to keep (0 = unlimited)
    pub fn new(
        dir: impl Into<PathBuf>,
        prefix: impl Into<String>,
        rotation: RotationStrategy,
        max_files: usize,
    ) -> std::io::Result<Self> {
        let dir = dir.into();
        fs::create_dir_all(&dir)?;
        let mut drain = Self {
            dir,
            prefix: prefix.into(),
            rotation,
            max_files,
            writer: None,
            current_bytes: 0,
            current_slot: 0,
        };
        drain.open_writer()?;
        Ok(drain)
    }

    fn current_path(&self) -> PathBuf {
        self.dir.join(format!("{}.log", self.prefix))
    }

    fn rotation_slot_now(&self) -> u64 {
        let secs = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        match self.rotation {
            RotationStrategy::Daily => secs / 86400,
            RotationStrategy::Hourly => secs / 3600,
            RotationStrategy::Size { .. } => 0,
        }
    }

    fn open_writer(&mut self) -> std::io::Result<()> {
        let path = self.current_path();
        let file = OpenOptions::new().create(true).append(true).open(&path)?;
        self.current_bytes = file.metadata()?.len();
        self.current_slot = self.rotation_slot_now();
        self.writer = Some(BufWriter::new(file));
        Ok(())
    }

    fn should_rotate(&self, additional_bytes: u64) -> bool {
        match self.rotation {
            RotationStrategy::Daily | RotationStrategy::Hourly => {
                self.rotation_slot_now() != self.current_slot
            }
            RotationStrategy::Size { max_bytes } => {
                self.current_bytes + additional_bytes > max_bytes
            }
        }
    }

    fn rotate(&mut self) -> std::io::Result<()> {
        // Flush and drop current writer
        if let Some(mut w) = self.writer.take() {
            let _ = w.flush();
        }

        // Rename current file with timestamp suffix
        let current = self.current_path();
        if current.exists() {
            let ts = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();
            let rotated = self.dir.join(format!("{}.{}.log", self.prefix, ts));
            let _ = fs::rename(&current, &rotated);
        }

        // Prune old files
        if self.max_files > 0 {
            self.prune_old_files();
        }

        self.current_bytes = 0;
        self.open_writer()
    }

    fn prune_old_files(&self) {
        let prefix = format!("{}.", self.prefix);
        if let Ok(entries) = fs::read_dir(&self.dir) {
            let mut files: Vec<PathBuf> = entries
                .flatten()
                .map(|e| e.path())
                .filter(|p| {
                    p.file_name()
                        .and_then(|n| n.to_str())
                        .map(|n| n.starts_with(&prefix) && n.ends_with(".log"))
                        .unwrap_or(false)
                })
                .collect();

            // Sort by modification time (oldest first)
            files.sort_by_key(|p| {
                p.metadata()
                    .and_then(|m| m.modified())
                    .unwrap_or(UNIX_EPOCH)
            });

            // Remove oldest to keep only max_files
            while files.len() > self.max_files {
                let _ = fs::remove_file(files.remove(0));
            }
        }
    }

    fn write_slot(&mut self, slot: &LogSlot) -> std::io::Result<()> {
        let level = LogLevel::from(slot.header.level);
        let category = LogCategory::from(slot.header.category);

        let mut record = serde_json::json!({
            "ts":       slot.header.timestamp_ns,
            "level":    level.to_string(),
            "category": category.to_string(),
            "svc":      slot.header.service_hash,
            "handler":  slot.header.handler_hash,
            "pid":      slot.header.process_id,
            "trace_id": slot.header.trace_id,
        });

        if slot.payload[0] != 0 {
            if let Ok(val) = rmp_serde::from_slice::<serde_json::Value>(&slot.payload) {
                if let Some(obj) = record.as_object_mut() {
                    obj.insert("data".to_string(), val);
                }
            }
        }

        let line = serde_json::to_string(&record)?;
        let line_bytes = (line.len() + 1) as u64; // +1 for newline

        if self.should_rotate(line_bytes) {
            self.rotate()?;
        }

        if let Some(w) = self.writer.as_mut() {
            writeln!(w, "{}", line)?;
            self.current_bytes += line_bytes;
        }

        Ok(())
    }
}

#[async_trait]
impl LogDrain for FileDrain {
    fn name(&self) -> &'static str {
        "file"
    }

    async fn flush(
        &mut self,
        batch: &[LogSlot],
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        for slot in batch {
            self.write_slot(slot)?;
        }
        // Flush writer buffer
        if let Some(w) = self.writer.as_mut() {
            w.flush()?;
        }
        Ok(())
    }

    async fn shutdown(&mut self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        if let Some(mut w) = self.writer.take() {
            w.flush()?;
        }
        Ok(())
    }
}
