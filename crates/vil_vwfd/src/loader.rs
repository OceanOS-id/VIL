//! Loader — scan workflows/ directory at startup, compile each VWFD → VilwGraph.

use crate::compiler;
use crate::graph::VilwGraph;
use std::path::Path;

/// Result of loading a directory of workflows.
#[derive(Debug)]
pub struct LoadResult {
    pub graphs: Vec<VilwGraph>,
    pub errors: Vec<LoadError>,
}

#[derive(Debug)]
pub struct LoadError {
    pub file: String,
    pub error: String,
}

/// Load all VWFD YAML files from a directory. Compiles each to VilwGraph.
/// Files that fail to compile are collected in errors (not fatal).
pub fn load_dir(dir: &str) -> LoadResult {
    let path = Path::new(dir);
    let mut result = LoadResult { graphs: Vec::new(), errors: Vec::new() };

    if !path.exists() || !path.is_dir() {
        result.errors.push(LoadError {
            file: dir.into(),
            error: format!("directory '{}' does not exist", dir),
        });
        return result;
    }

    let entries = match std::fs::read_dir(path) {
        Ok(e) => e,
        Err(e) => {
            result.errors.push(LoadError { file: dir.into(), error: e.to_string() });
            return result;
        }
    };

    for entry in entries {
        let entry = match entry {
            Ok(e) => e,
            Err(_) => continue,
        };

        let file_path = entry.path();
        let ext = file_path.extension().and_then(|e| e.to_str()).unwrap_or("");

        // Only load .yaml and .yml files
        if ext != "yaml" && ext != "yml" && ext != "vwfd" {
            continue;
        }

        let file_name = file_path.display().to_string();
        match load_file(&file_path) {
            Ok(graph) => {
                tracing::info!("Loaded workflow: {} (id={}, {} nodes, route={:?})",
                    file_name, graph.id, graph.node_count(), graph.webhook_route);
                result.graphs.push(graph);
            }
            Err(e) => {
                tracing::warn!("Failed to load {}: {}", file_name, e);
                result.errors.push(LoadError { file: file_name, error: e });
            }
        }
    }

    result
}

/// Load single VWFD file → VilwGraph.
pub fn load_file(path: &Path) -> Result<VilwGraph, String> {
    let yaml = std::fs::read_to_string(path)
        .map_err(|e| format!("read {}: {}", path.display(), e))?;
    compiler::compile(&yaml)
        .map_err(|e| e.to_string())
}

/// Load from YAML string (for embedded workflows from macros).
pub fn load_yaml(yaml: &str) -> Result<VilwGraph, String> {
    compiler::compile(yaml)
        .map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_load_yaml() {
        let yaml = r#"
version: "3.0"
metadata:
  id: loader-test
spec:
  activities:
    - id: trigger
      activity_type: Trigger
      trigger_config:
        trigger_type: webhook
        webhook_config: { path: /loader }
      output_variable: trigger_payload
    - id: end
      activity_type: End
  flows:
    - { id: f1, from: { node: trigger }, to: { node: end } }
"#;
        let graph = load_yaml(yaml).unwrap();
        assert_eq!(graph.id, "loader-test");
        assert_eq!(graph.webhook_route, Some("/loader".into()));
    }

    #[test]
    fn test_load_dir_nonexistent() {
        let result = load_dir("/nonexistent/dir");
        assert!(result.graphs.is_empty());
        assert!(!result.errors.is_empty());
    }

    #[test]
    fn test_load_dir_with_files() {
        // Create temp dir with a workflow file
        let dir = std::env::temp_dir().join("vil_vwfd_test_loader");
        let _ = std::fs::create_dir_all(&dir);
        std::fs::write(dir.join("test.yaml"), r#"
version: "3.0"
metadata:
  id: dir-test
spec:
  activities:
    - id: trigger
      activity_type: Trigger
      trigger_config:
        trigger_type: webhook
        webhook_config: { path: /dir-test }
      output_variable: trigger_payload
    - id: end
      activity_type: End
  flows:
    - { id: f1, from: { node: trigger }, to: { node: end } }
"#).unwrap();

        let result = load_dir(dir.to_str().unwrap());
        assert_eq!(result.graphs.len(), 1);
        assert_eq!(result.graphs[0].id, "dir-test");
        assert!(result.errors.is_empty());

        // Cleanup
        let _ = std::fs::remove_dir_all(&dir);
    }
}
