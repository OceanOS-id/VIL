// ── vil_index_updater ── N06: Incremental Index Update ────────────
//!
//! Write-ahead log and batched incremental updater for vector indices.
//! Collects insert/delete/update operations and flushes them in batches.

pub mod updater;
pub mod wal;

pub use updater::{FlushResult, IncrementalUpdater};
pub use wal::{WalEntry, WriteAheadLog};

// VIL integration layer
pub mod handlers;
pub mod pipeline_sse;
pub mod plugin;
pub mod vil_semantic;

pub use plugin::IndexUpdaterPlugin;
pub use vil_semantic::{IndexUpdateEvent, IndexUpdateFault, IndexUpdaterState};
