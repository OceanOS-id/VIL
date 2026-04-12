//! Durability — redb-backed checkpoint store for VIL VWFD execution.
//!
//! Single-tier (redb only, no papaya/WAL tiered store like VFlow).
//! Checkpoint after each activity. Recovery on startup.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionState {
    pub exec_id: String,
    pub workflow_id: String,
    pub status: ExecStatus,
    pub current_node: String,
    pub step: u32,
    pub completed_nodes: Vec<String>,
    pub variables: HashMap<String, Value>,
    pub input: Value,
    pub started_at: u64,
    pub updated_at: u64,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExecStatus {
    Running,
    Completed,
    Failed,
    Compensating,
}

impl ExecStatus {
    pub fn is_terminal(&self) -> bool {
        matches!(self, ExecStatus::Completed | ExecStatus::Failed)
    }
}

// redb table definition
const EXEC_TABLE: redb::TableDefinition<&str, &[u8]> =
    redb::TableDefinition::new("executions");

/// Durability store — redb-backed (persistent) or in-memory (testing).
pub struct DurabilityStore {
    db: Option<redb::Database>,
    /// In-memory fallback when no db path provided.
    mem: std::sync::RwLock<HashMap<String, ExecutionState>>,
}

impl DurabilityStore {
    /// Create persistent store backed by redb at given path.
    pub fn persistent(path: impl AsRef<Path>) -> Result<Self, String> {
        let db = redb::Database::create(path.as_ref())
            .map_err(|e| format!("redb open: {}", e))?;
        Ok(Self {
            db: Some(db),
            mem: std::sync::RwLock::new(HashMap::new()),
        })
    }

    /// Create in-memory store (no persistence — for testing).
    pub fn in_memory() -> Self {
        Self {
            db: None,
            mem: std::sync::RwLock::new(HashMap::new()),
        }
    }

    fn write_state(&self, state: &ExecutionState) {
        if let Some(ref db) = self.db {
            let bytes = serde_json::to_vec(state).unwrap_or_default();
            if let Ok(txn) = db.begin_write() {
                if let Ok(mut table) = txn.open_table(EXEC_TABLE) {
                    let _ = table.insert(state.exec_id.as_str(), bytes.as_slice());
                }
                let _ = txn.commit();
            }
        }
        self.mem.write().unwrap().insert(state.exec_id.clone(), state.clone());
    }

    fn read_state(&self, exec_id: &str) -> Option<ExecutionState> {
        // Memory-first (always in sync)
        self.mem.read().unwrap().get(exec_id).cloned()
    }

    pub fn begin(&self, exec_id: &str, workflow_id: &str, input: &Value) {
        let state = ExecutionState {
            exec_id: exec_id.into(),
            workflow_id: workflow_id.into(),
            status: ExecStatus::Running,
            current_node: String::new(),
            step: 0,
            completed_nodes: Vec::new(),
            variables: HashMap::new(),
            input: input.clone(),
            started_at: now_ms(),
            updated_at: now_ms(),
            error: None,
        };
        self.write_state(&state);
    }

    pub fn checkpoint(
        &self,
        exec_id: &str,
        node_id: &str,
        step: u32,
        variables: &HashMap<String, Value>,
    ) {
        let mut states = self.mem.write().unwrap();
        if let Some(state) = states.get_mut(exec_id) {
            state.current_node = node_id.into();
            state.step = step;
            state.completed_nodes.push(node_id.into());
            state.variables = variables.clone();
            state.updated_at = now_ms();
            // Persist to redb
            if let Some(ref db) = self.db {
                let bytes = serde_json::to_vec(&*state).unwrap_or_default();
                if let Ok(txn) = db.begin_write() {
                    if let Ok(mut table) = txn.open_table(EXEC_TABLE) {
                        let _ = table.insert(exec_id, bytes.as_slice());
                    }
                    let _ = txn.commit();
                }
            }
        }
    }

    pub fn complete(&self, exec_id: &str) {
        self.update_status(exec_id, ExecStatus::Completed, None);
    }

    pub fn fail(&self, exec_id: &str, error: &str) {
        self.update_status(exec_id, ExecStatus::Failed, Some(error));
    }

    pub fn set_compensating(&self, exec_id: &str) {
        self.update_status(exec_id, ExecStatus::Compensating, None);
    }

    fn update_status(&self, exec_id: &str, status: ExecStatus, error: Option<&str>) {
        let mut states = self.mem.write().unwrap();
        if let Some(state) = states.get_mut(exec_id) {
            state.status = status;
            state.updated_at = now_ms();
            if let Some(e) = error {
                state.error = Some(e.into());
            }
            // Persist
            if let Some(ref db) = self.db {
                let bytes = serde_json::to_vec(&*state).unwrap_or_default();
                if let Ok(txn) = db.begin_write() {
                    if let Ok(mut table) = txn.open_table(EXEC_TABLE) {
                        let _ = table.insert(exec_id, bytes.as_slice());
                    }
                    let _ = txn.commit();
                }
            }
        }
    }

    pub fn get(&self, exec_id: &str) -> Option<ExecutionState> {
        self.read_state(exec_id)
    }

    pub fn list_incomplete(&self) -> Vec<ExecutionState> {
        self.mem.read().unwrap().values()
            .filter(|s| !s.status.is_terminal())
            .cloned()
            .collect()
    }

    pub fn list_all(&self) -> Vec<ExecutionState> {
        self.mem.read().unwrap().values().cloned().collect()
    }

    pub fn purge_completed(&self) -> usize {
        let mut states = self.mem.write().unwrap();
        let to_remove: Vec<String> = states.iter()
            .filter(|(_, s)| s.status.is_terminal())
            .map(|(k, _)| k.clone())
            .collect();
        let count = to_remove.len();
        // Remove from redb
        if let Some(ref db) = self.db {
            if let Ok(txn) = db.begin_write() {
                if let Ok(mut table) = txn.open_table(EXEC_TABLE) {
                    for k in &to_remove {
                        let _ = table.remove(k.as_str());
                    }
                }
                let _ = txn.commit();
            }
        }
        // Remove from memory
        for k in &to_remove {
            states.remove(k);
        }
        count
    }

    /// Load all states from redb into memory (call on startup for recovery).
    pub fn recover(&self) -> usize {
        let Some(ref db) = self.db else { return 0 };
        let Ok(txn) = db.begin_read() else { return 0 };
        let Ok(table) = txn.open_table(EXEC_TABLE) else { return 0 };

        let mut states = self.mem.write().unwrap();
        let mut count = 0;
        if let Ok(iter) = table.range::<&str>(..) {
            for entry in iter {
                if let Ok(guard) = entry {
                    let key: &str = guard.0.value();
                    let val: &[u8] = guard.1.value();
                    if let Ok(state) = serde_json::from_slice::<ExecutionState>(val) {
                        states.insert(key.to_string(), state);
                        count += 1;
                    }
                }
            }
        }
        count
    }
}

fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_lifecycle() {
        let store = DurabilityStore::in_memory();
        store.begin("e1", "wf1", &json!({"name": "test"}));

        let s = store.get("e1").unwrap();
        assert_eq!(s.status, ExecStatus::Running);

        store.checkpoint("e1", "step1", 1, &HashMap::new());
        store.checkpoint("e1", "step2", 2, &HashMap::new());

        let s = store.get("e1").unwrap();
        assert_eq!(s.step, 2);
        assert_eq!(s.completed_nodes.len(), 2);

        store.complete("e1");
        let s = store.get("e1").unwrap();
        assert_eq!(s.status, ExecStatus::Completed);
        assert!(store.list_incomplete().is_empty());
    }

    #[test]
    fn test_fail_and_compensate() {
        let store = DurabilityStore::in_memory();
        store.begin("e2", "wf1", &json!({}));
        store.checkpoint("e2", "step1", 1, &HashMap::new());
        store.fail("e2", "timeout");

        let s = store.get("e2").unwrap();
        assert_eq!(s.status, ExecStatus::Failed);
        assert_eq!(s.error.as_deref(), Some("timeout"));
    }

    #[test]
    fn test_list_incomplete() {
        let store = DurabilityStore::in_memory();
        store.begin("e1", "wf1", &json!({}));
        store.begin("e2", "wf1", &json!({}));
        store.complete("e1");

        assert_eq!(store.list_incomplete().len(), 1);
        assert_eq!(store.list_incomplete()[0].exec_id, "e2");
    }

    #[test]
    fn test_purge() {
        let store = DurabilityStore::in_memory();
        store.begin("e1", "wf1", &json!({}));
        store.begin("e2", "wf1", &json!({}));
        store.complete("e1");
        store.fail("e2", "err");

        assert_eq!(store.purge_completed(), 2);
        assert!(store.list_all().is_empty());
    }

    #[test]
    fn test_redb_persistent() {
        let dir = std::env::temp_dir().join("vil_vwfd_test_redb");
        let _ = std::fs::remove_file(&dir);

        let store = DurabilityStore::persistent(&dir).unwrap();
        store.begin("p1", "wf1", &json!({"key": "val"}));
        store.checkpoint("p1", "node1", 1, &HashMap::new());
        store.complete("p1");

        // Verify in redb
        let s = store.get("p1").unwrap();
        assert_eq!(s.status, ExecStatus::Completed);

        // Simulate restart: drop first store, then reopen
        drop(store);
        let store2 = DurabilityStore::persistent(&dir).unwrap();
        let recovered = store2.recover();
        assert_eq!(recovered, 1);
        let s = store2.get("p1").unwrap();
        assert_eq!(s.status, ExecStatus::Completed);
        assert_eq!(s.completed_nodes, vec!["node1"]);

        let _ = std::fs::remove_file(&dir);
    }
}
