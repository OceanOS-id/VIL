// ── N06: Incremental Updater ────────────────────────────────────────
use crate::wal::{WalEntry, WriteAheadLog};
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};

/// Result of a flush operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlushResult {
    pub inserts: usize,
    pub deletes: usize,
    pub updates: usize,
    pub total_flushed: usize,
}

/// Incremental index updater — collects WAL entries and flushes in batches.
pub struct IncrementalUpdater {
    wal: Mutex<WriteAheadLog>,
    pub batch_size: usize,
}

impl IncrementalUpdater {
    pub fn new(batch_size: usize) -> Self {
        Self {
            wal: Mutex::new(WriteAheadLog::new()),
            batch_size,
        }
    }

    /// Append an entry to the WAL.
    pub fn append(&self, entry: WalEntry) {
        self.wal.lock().append(entry);
    }

    /// Number of pending (unflushed) entries.
    pub fn pending_count(&self) -> usize {
        self.wal.lock().len()
    }

    /// Returns true if pending entries >= batch_size.
    pub fn should_flush(&self) -> bool {
        self.pending_count() >= self.batch_size
    }

    /// Flush all pending entries and return the result.
    /// In a real implementation this would apply changes to the vector index.
    pub fn flush(&self) -> FlushResult {
        let entries = self.wal.lock().drain();

        let mut inserts = 0usize;
        let mut deletes = 0usize;
        let mut updates = 0usize;

        for entry in &entries {
            match entry {
                WalEntry::Insert { .. } => inserts += 1,
                WalEntry::Delete { .. } => deletes += 1,
                WalEntry::Update { .. } => updates += 1,
            }
        }

        FlushResult {
            inserts,
            deletes,
            updates,
            total_flushed: entries.len(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn append_and_pending() {
        let updater = IncrementalUpdater::new(10);
        updater.append(WalEntry::insert("a", vec![1.0]));
        updater.append(WalEntry::insert("b", vec![2.0]));
        assert_eq!(updater.pending_count(), 2);
    }

    #[test]
    fn flush_returns_correct_counts() {
        let updater = IncrementalUpdater::new(10);
        updater.append(WalEntry::insert("a", vec![1.0]));
        updater.append(WalEntry::insert("b", vec![2.0]));
        updater.append(WalEntry::delete("c"));
        updater.append(WalEntry::update("d", vec![3.0]));

        let result = updater.flush();
        assert_eq!(result.inserts, 2);
        assert_eq!(result.deletes, 1);
        assert_eq!(result.updates, 1);
        assert_eq!(result.total_flushed, 4);
    }

    #[test]
    fn flush_clears_pending() {
        let updater = IncrementalUpdater::new(10);
        updater.append(WalEntry::insert("a", vec![1.0]));
        updater.flush();
        assert_eq!(updater.pending_count(), 0);
    }

    #[test]
    fn empty_flush() {
        let updater = IncrementalUpdater::new(10);
        let result = updater.flush();
        assert_eq!(result.total_flushed, 0);
        assert_eq!(result.inserts, 0);
    }

    #[test]
    fn should_flush_threshold() {
        let updater = IncrementalUpdater::new(3);
        updater.append(WalEntry::insert("a", vec![1.0]));
        updater.append(WalEntry::insert("b", vec![2.0]));
        assert!(!updater.should_flush());
        updater.append(WalEntry::insert("c", vec![3.0]));
        assert!(updater.should_flush());
    }

    #[test]
    fn multiple_flushes() {
        let updater = IncrementalUpdater::new(2);
        updater.append(WalEntry::insert("a", vec![1.0]));
        updater.append(WalEntry::delete("b"));
        let r1 = updater.flush();
        assert_eq!(r1.total_flushed, 2);

        updater.append(WalEntry::update("c", vec![3.0]));
        let r2 = updater.flush();
        assert_eq!(r2.total_flushed, 1);
        assert_eq!(r2.updates, 1);
    }
}
