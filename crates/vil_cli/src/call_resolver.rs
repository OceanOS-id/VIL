//! Workflow call resolver — loads and validates called workflow YAML files.
//!
//! When a task has `call: workflows/payment.vil.yaml`, this module:
//! 1. Resolves the path relative to the parent YAML directory
//! 2. Parses the called YAML into WorkflowManifest
//! 3. Extracts the workflow's contract (input/output/error)
//! 4. Validates compatibility with the caller's context

use crate::manifest::{WorkflowDagManifest, WorkflowManifest};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Resolved call target — a parsed workflow with its file path.
#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct ResolvedCall {
    pub path: PathBuf,
    pub manifest: WorkflowManifest,
    pub workflow: WorkflowDagManifest,
    pub input_type: Option<String>,
    pub output_type: Option<String>,
    pub error_type: Option<String>,
}

/// Resolve all `call:` references in a manifest.
/// Returns a map of call_path → ResolvedCall.
pub fn resolve_all_calls(
    manifest: &WorkflowManifest,
    base_dir: &Path,
) -> Result<HashMap<String, ResolvedCall>, Vec<String>> {
    let mut resolved = HashMap::new();
    let mut errors = Vec::new();

    for (_wf_name, wf) in &manifest.workflows {
        for task in &wf.tasks {
            if let Some(call_path) = &task.call {
                if resolved.contains_key(call_path) {
                    continue; // already resolved
                }
                match resolve_single_call(base_dir, call_path) {
                    Ok(r) => {
                        resolved.insert(call_path.clone(), r);
                    }
                    Err(e) => errors.push(e),
                }
            }
        }

        // Also check on_complete targets
        if let Some(on_complete) = &wf.on_complete {
            for path in [&on_complete.success, &on_complete.failure]
                .into_iter()
                .flatten()
            {
                if !resolved.contains_key(path) {
                    match resolve_single_call(base_dir, path) {
                        Ok(r) => {
                            resolved.insert(path.clone(), r);
                        }
                        Err(e) => errors.push(e),
                    }
                }
            }
        }
    }

    if errors.is_empty() {
        Ok(resolved)
    } else {
        Err(errors)
    }
}

/// Resolve a single call path.
fn resolve_single_call(base_dir: &Path, call_path: &str) -> Result<ResolvedCall, String> {
    let full_path = base_dir.join(call_path);

    if !full_path.exists() {
        return Err(format!(
            "call: '{}' — file not found at {}",
            call_path,
            full_path.display()
        ));
    }

    let manifest = WorkflowManifest::from_file(
        full_path
            .to_str()
            .ok_or_else(|| format!("invalid path: {}", full_path.display()))?,
    )?;

    // Find the first (or only) workflow in the called manifest.
    // Convention: a called file should have exactly one workflow, or a default one.
    let workflow = if manifest.workflows.len() == 1 {
        manifest
            .workflows
            .values()
            .next()
            .cloned()
            .unwrap_or_default()
    } else if let Some(wf) = manifest.workflows.get(&manifest.name) {
        wf.clone()
    } else {
        // No named workflow — treat the whole manifest as a flat workflow
        WorkflowDagManifest::default()
    };

    Ok(ResolvedCall {
        path: full_path,
        input_type: workflow.input.clone(),
        output_type: workflow.output.clone(),
        error_type: workflow.error.clone(),
        workflow,
        manifest,
    })
}

/// Scan a directory for all `*.vil.yaml` files and build a call graph.
/// Returns (nodes, edges) where nodes are file paths and edges are call references.
pub fn scan_call_graph(dir: &Path) -> Result<CallGraph, String> {
    let mut nodes = Vec::new();
    let mut edges = Vec::new();

    let entries = std::fs::read_dir(dir)
        .map_err(|e| format!("Failed to read directory '{}': {}", dir.display(), e))?;

    // Collect all YAML files
    let mut yaml_files: Vec<PathBuf> = Vec::new();
    collect_yaml_files(dir, &mut yaml_files)?;

    for file_path in &yaml_files {
        let rel_path = file_path.strip_prefix(dir).unwrap_or(file_path);
        let name = rel_path.to_string_lossy().to_string();

        // Parse the manifest
        let content = std::fs::read_to_string(file_path)
            .map_err(|e| format!("Failed to read '{}': {}", file_path.display(), e))?;

        // Try to parse — skip files that aren't valid manifests
        let manifest: Result<WorkflowManifest, _> = serde_yaml::from_str(&content);
        let manifest = match manifest {
            Ok(m) => m,
            Err(_) => continue,
        };

        let node_count = manifest.nodes.len();
        let wf_count = manifest.workflows.len();

        nodes.push(CallGraphNode {
            name: name.clone(),
            path: file_path.clone(),
            node_count,
            workflow_count: wf_count,
        });

        // Find all call: references
        for (_wf_name, wf) in &manifest.workflows {
            for task in &wf.tasks {
                if let Some(call_path) = &task.call {
                    edges.push(CallGraphEdge {
                        from: name.clone(),
                        to: call_path.clone(),
                        task_id: task.id.clone(),
                    });
                }
            }
            if let Some(on_complete) = &wf.on_complete {
                if let Some(success) = &on_complete.success {
                    edges.push(CallGraphEdge {
                        from: name.clone(),
                        to: success.clone(),
                        task_id: "_on_success".into(),
                    });
                }
                if let Some(failure) = &on_complete.failure {
                    edges.push(CallGraphEdge {
                        from: name.clone(),
                        to: failure.clone(),
                        task_id: "_on_failure".into(),
                    });
                }
            }
        }
    }

    let _ = entries; // consumed above indirectly
    Ok(CallGraph { nodes, edges })
}

fn collect_yaml_files(dir: &Path, out: &mut Vec<PathBuf>) -> Result<(), String> {
    let entries = std::fs::read_dir(dir)
        .map_err(|e| format!("Failed to read directory '{}': {}", dir.display(), e))?;

    for entry in entries {
        let entry = entry.map_err(|e| format!("Directory entry error: {}", e))?;
        let path = entry.path();
        if path.is_dir() {
            collect_yaml_files(&path, out)?;
        } else if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
            if name.ends_with(".vil.yaml") || name.ends_with(".vil.yml") {
                out.push(path);
            }
        }
    }
    Ok(())
}

/// A file-level call graph.
#[derive(Debug, Clone)]
pub struct CallGraph {
    pub nodes: Vec<CallGraphNode>,
    pub edges: Vec<CallGraphEdge>,
}

#[derive(Debug, Clone)]
pub struct CallGraphNode {
    pub name: String,
    pub path: PathBuf,
    pub node_count: usize,
    pub workflow_count: usize,
}

#[derive(Debug, Clone)]
pub struct CallGraphEdge {
    pub from: String,
    pub to: String,
    pub task_id: String,
}
