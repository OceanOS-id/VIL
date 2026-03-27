// ── N06: Write-Ahead Log ────────────────────────────────────────────
use serde::{Deserialize, Serialize};

/// A single WAL entry representing an index mutation.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum WalEntry {
    Insert { id: String, embedding: Vec<f32> },
    Delete { id: String },
    Update { id: String, embedding: Vec<f32> },
}

impl WalEntry {
    pub fn insert(id: impl Into<String>, embedding: Vec<f32>) -> Self {
        WalEntry::Insert {
            id: id.into(),
            embedding,
        }
    }

    pub fn delete(id: impl Into<String>) -> Self {
        WalEntry::Delete { id: id.into() }
    }

    pub fn update(id: impl Into<String>, embedding: Vec<f32>) -> Self {
        WalEntry::Update {
            id: id.into(),
            embedding,
        }
    }

    /// Extract the id from any variant.
    pub fn id(&self) -> &str {
        match self {
            WalEntry::Insert { id, .. } => id,
            WalEntry::Delete { id } => id,
            WalEntry::Update { id, .. } => id,
        }
    }
}

/// Append-only write-ahead log.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct WriteAheadLog {
    pub entries: Vec<WalEntry>,
}

impl WriteAheadLog {
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
        }
    }

    /// Append an entry to the log.
    pub fn append(&mut self, entry: WalEntry) {
        self.entries.push(entry);
    }

    /// Number of entries in the log.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Drain all entries, returning them and clearing the log.
    pub fn drain(&mut self) -> Vec<WalEntry> {
        std::mem::take(&mut self.entries)
    }

    /// Serialize the WAL to JSON.
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }

    /// Deserialize from JSON.
    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wal_append_and_len() {
        let mut wal = WriteAheadLog::new();
        wal.append(WalEntry::insert("doc1", vec![1.0, 2.0]));
        wal.append(WalEntry::delete("doc2"));
        assert_eq!(wal.len(), 2);
    }

    #[test]
    fn wal_drain() {
        let mut wal = WriteAheadLog::new();
        wal.append(WalEntry::insert("a", vec![1.0]));
        wal.append(WalEntry::update("b", vec![2.0]));
        let drained = wal.drain();
        assert_eq!(drained.len(), 2);
        assert!(wal.is_empty());
    }

    #[test]
    fn wal_ordering_preserved() {
        let mut wal = WriteAheadLog::new();
        wal.append(WalEntry::insert("first", vec![1.0]));
        wal.append(WalEntry::delete("second"));
        wal.append(WalEntry::update("third", vec![3.0]));
        assert_eq!(wal.entries[0].id(), "first");
        assert_eq!(wal.entries[1].id(), "second");
        assert_eq!(wal.entries[2].id(), "third");
    }

    #[test]
    fn wal_json_roundtrip() {
        let mut wal = WriteAheadLog::new();
        wal.append(WalEntry::insert("x", vec![0.5, 0.5]));
        let json = wal.to_json().unwrap();
        let wal2 = WriteAheadLog::from_json(&json).unwrap();
        assert_eq!(wal2.len(), 1);
        assert_eq!(wal2.entries[0], wal.entries[0]);
    }
}
