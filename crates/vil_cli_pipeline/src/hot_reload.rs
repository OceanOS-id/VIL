//! Script hot-reload file watcher.
//!
//! Watches script files referenced by `code: { hot_reload: true }` nodes/tasks.
//! On file change: re-read → validate → atomic swap to new version.
//! Old script continues serving in-flight requests.
//!
//! Also supports `vil viz --watch` for YAML auto-re-render.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;

/// A watched file entry.
#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct WatchEntry {
    pub path: PathBuf,
    pub kind: WatchKind,
    pub last_modified: u64,
}

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq)]
pub enum WatchKind {
    Script { runtime: String, node_name: String },
    Yaml,
}

/// Simple file watcher using polling (no external deps required).
/// For production, replace with `notify` crate for OS-level events.
pub struct FileWatcher {
    entries: Vec<WatchEntry>,
    poll_interval: Duration,
    running: Arc<AtomicBool>,
    reload_count: Arc<AtomicU64>,
}

impl FileWatcher {
    pub fn new(poll_interval_ms: u64) -> Self {
        Self {
            entries: Vec::new(),
            poll_interval: Duration::from_millis(poll_interval_ms),
            running: Arc::new(AtomicBool::new(false)),
            reload_count: Arc::new(AtomicU64::new(0)),
        }
    }

    /// Add a file to watch.
    pub fn watch(&mut self, path: impl Into<PathBuf>, kind: WatchKind) {
        let path = path.into();
        let last_modified = file_mtime(&path).unwrap_or(0);
        self.entries.push(WatchEntry {
            path,
            kind,
            last_modified,
        });
    }

    /// Start watching in a background thread.
    /// Calls `on_change` for each changed file.
    pub fn start<F>(&self, on_change: F) -> std::thread::JoinHandle<()>
    where
        F: Fn(&WatchEntry) + Send + 'static,
    {
        let entries = self.entries.clone();
        let interval = self.poll_interval;
        let running = self.running.clone();
        let reload_count = self.reload_count.clone();

        running.store(true, Ordering::SeqCst);

        std::thread::spawn(move || {
            let mut state: Vec<u64> = entries.iter().map(|e| e.last_modified).collect();

            while running.load(Ordering::SeqCst) {
                std::thread::sleep(interval);

                for (i, entry) in entries.iter().enumerate() {
                    if let Some(mtime) = file_mtime(&entry.path) {
                        if mtime != state[i] {
                            state[i] = mtime;
                            reload_count.fetch_add(1, Ordering::SeqCst);
                            on_change(entry);
                        }
                    }
                }
            }
        })
    }

    /// Stop the watcher.
    #[allow(dead_code)]
    pub fn stop(&self) {
        self.running.store(false, Ordering::SeqCst);
    }

    #[allow(dead_code)]
    pub fn reload_count(&self) -> u64 {
        self.reload_count.load(Ordering::SeqCst)
    }
}

/// Collect all hot-reloadable script paths from a manifest.
#[allow(dead_code)]
pub fn collect_hot_reload_paths(
    manifest: &vil_cli_core::manifest::WorkflowManifest,
) -> Vec<(PathBuf, String, String)> {
    let mut paths = Vec::new(); // (path, runtime, node/task name)

    // Check nodes
    for (name, node) in &manifest.nodes {
        if let Some(code) = &node.code {
            if code.hot_reload == Some(true) {
                if let Some(src) = &code.source {
                    let runtime = code.runtime.as_deref().unwrap_or("lua");
                    paths.push((PathBuf::from(src), runtime.to_string(), name.clone()));
                }
            }
        }
    }

    // Check workflow tasks
    for (wf_name, wf) in &manifest.workflows {
        for task in &wf.tasks {
            if let Some(code) = &task.code {
                if code.hot_reload == Some(true) {
                    if let Some(src) = &code.source {
                        let runtime = code.runtime.as_deref().unwrap_or("lua");
                        let id = format!("{}.{}", wf_name, task.id);
                        paths.push((PathBuf::from(src), runtime.to_string(), id));
                    }
                }
            }
        }
    }

    paths
}

fn file_mtime(path: &Path) -> Option<u64> {
    std::fs::metadata(path)
        .ok()
        .and_then(|m| m.modified().ok())
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|d| d.as_secs())
}
