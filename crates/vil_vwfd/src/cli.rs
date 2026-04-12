//! CLI functions — called by `vil` CLI for VWFD commands.
//!
//! Integration: vil_cli/src/main.rs adds these as subcommands:
//!   vil compile --vwfd <file>   → compile_vwfd()
//!   vil lint <file>             → lint_vwfd()
//!   vil lint --all              → lint_dir()
//!   vil export --vwfd           → export_vwfd()

use crate::compiler;
use std::path::Path;

// ── vil compile --vwfd ──

pub struct CompileResult {
    pub id: String,
    pub node_count: usize,
    pub route: Option<String>,
    pub bytes: usize,
    pub duration_ms: u64,
}

/// Compile single VWFD file. Validates, compiles vil_query, produces VILW.
pub fn compile_vwfd(path: &str) -> Result<CompileResult, String> {
    let start = std::time::Instant::now();
    let yaml = std::fs::read_to_string(path)
        .map_err(|e| format!("read {}: {}", path, e))?;

    let graph = compiler::compile(&yaml)
        .map_err(|e| e.to_string())?;

    let bytes = graph.to_bytes();
    let dur = start.elapsed().as_millis() as u64;

    Ok(CompileResult {
        id: graph.id.clone(),
        node_count: graph.node_count(),
        route: graph.webhook_route.clone(),
        bytes: bytes.len(),
        duration_ms: dur,
    })
}

/// Compile all VWFD files in directory.
pub fn compile_all(dir: &str) -> Vec<Result<CompileResult, String>> {
    let mut results = Vec::new();
    let path = Path::new(dir);
    if !path.is_dir() {
        results.push(Err(format!("{} is not a directory", dir)));
        return results;
    }

    if let Ok(entries) = std::fs::read_dir(path) {
        for entry in entries.flatten() {
            let ext = entry.path().extension()
                .and_then(|e| e.to_str()).unwrap_or("").to_string();
            if ext == "yaml" || ext == "yml" || ext == "vwfd" {
                let file = entry.path().display().to_string();
                results.push(compile_vwfd(&file));
            }
        }
    }
    results
}

// ── vil lint ──

#[derive(Debug)]
pub struct LintResult {
    pub file: String,
    pub errors: Vec<LintIssue>,
    pub warnings: Vec<LintIssue>,
    pub infos: Vec<LintIssue>,
}

#[derive(Debug)]
pub struct LintIssue {
    pub code: String,
    pub message: String,
    pub location: Option<String>,
}

/// Lint single VWFD file with VIL Way rules.
pub fn lint_vwfd(path: &str) -> LintResult {
    let mut result = LintResult {
        file: path.into(),
        errors: Vec::new(),
        warnings: Vec::new(),
        infos: Vec::new(),
    };

    // Step 1: Parse
    let yaml = match std::fs::read_to_string(path) {
        Ok(y) => y,
        Err(e) => {
            result.errors.push(LintIssue {
                code: "VIL-L000".into(),
                message: format!("cannot read file: {}", e),
                location: None,
            });
            return result;
        }
    };

    lint_yaml(&yaml, &mut result);
    result
}

/// Lint YAML string (for testing without file).
pub fn lint_yaml(yaml: &str, result: &mut LintResult) {
    // Step 1: Parse YAML
    let doc: crate::spec::VwfdDocument = match serde_yaml::from_str(yaml) {
        Ok(d) => d,
        Err(e) => {
            result.errors.push(LintIssue {
                code: "VIL-L000".into(),
                message: format!("YAML parse error: {}", e),
                location: None,
            });
            return;
        }
    };

    // Step 2: Compile check (validates expressions, rejects unsupported)
    if let Err(e) = compiler::compile(yaml) {
        result.errors.push(LintIssue {
            code: "VIL-L000".into(),
            message: format!("compilation error: {}", e),
            location: e.location,
        });
        return;
    }

    // Step 3: VIL Way lint rules
    let activities = &doc.spec.activities;
    let flows = &doc.spec.flows;

    // Collect all node IDs
    let node_ids: Vec<&str> = activities.iter().map(|a| a.id.as_str()).collect();
    let control_ids: Vec<&str> = doc.spec.controls.as_ref()
        .map(|c| c.iter().map(|ctrl| ctrl.id.as_str()).collect())
        .unwrap_or_default();

    for act in activities {
        let loc = format!("activity.{}", act.id);

        // VIL-L001: External connector should have retry_policy
        if act.activity_type == "Connector" {
            if let Some(ref cc) = act.connector_config {
                let is_external = cc.connector_ref.as_deref()
                    .map(|r| r.contains("http") || r.contains("mq") || r.contains("storage"))
                    .unwrap_or(false);
                if is_external && cc.retry_policy.is_none() {
                    result.warnings.push(LintIssue {
                        code: "VIL-L001".into(),
                        message: "external connector should have retry_policy".into(),
                        location: Some(loc.clone()),
                    });
                }
            }
        }

        // VIL-L004: Connector should have timeout_ms
        if act.activity_type == "Connector" {
            if let Some(ref cc) = act.connector_config {
                if cc.timeout_ms.is_none() {
                    result.warnings.push(LintIssue {
                        code: "VIL-L004".into(),
                        message: "timeout_ms not set (default 30s assumed)".into(),
                        location: Some(loc.clone()),
                    });
                }
            }
        }

        // VIL-L003: Unknown connector_ref pattern
        if act.activity_type == "Connector" {
            if let Some(ref cc) = act.connector_config {
                if let Some(ref cref) = cc.connector_ref {
                    if !cref.starts_with("vastar.") {
                        result.errors.push(LintIssue {
                            code: "VIL-L003".into(),
                            message: format!("unknown connector_ref: '{}' (should start with 'vastar.')", cref),
                            location: Some(loc.clone()),
                        });
                    }
                }
            }
        }

        // VIL-L005: output_variable defined but not used in any downstream mapping
        if let Some(ref out_var) = act.output_variable {
            let used = activities.iter().any(|a| {
                a.input_mappings.as_ref().map(|maps| {
                    maps.iter().any(|m| {
                        m.source.as_ref().and_then(|s| s.source.as_ref())
                            .and_then(|v| v.as_str())
                            .map(|src| src.contains(out_var))
                            .unwrap_or(false)
                    })
                }).unwrap_or(false)
                || a.end_trigger_config.as_ref().and_then(|etc| etc.final_response.as_ref())
                    .and_then(|fr| fr.source.as_ref())
                    .and_then(|v| v.as_str())
                    .map(|src| src.contains(out_var))
                    .unwrap_or(false)
            });
            if !used && out_var != "trigger_payload" {
                result.infos.push(LintIssue {
                    code: "VIL-L005".into(),
                    message: format!("output_variable '{}' not referenced downstream", out_var),
                    location: Some(loc.clone()),
                });
            }
        }

        // VIL-L007: EndTrigger without end_activity on trigger
        if act.activity_type == "EndTrigger" {
            let trigger = activities.iter().find(|a| a.activity_type == "Trigger");
            if let Some(t) = trigger {
                let has_end_activity = t.trigger_config.as_ref()
                    .and_then(|tc| tc.end_activity.as_ref())
                    .is_some();
                if !has_end_activity {
                    result.warnings.push(LintIssue {
                        code: "VIL-L007".into(),
                        message: "EndTrigger exists but trigger has no end_activity reference".into(),
                        location: Some(loc.clone()),
                    });
                }
            }
        }

        // VIL-L009: Side-effect connector without compensation
        if act.activity_type == "Connector" && act.compensation.is_none() {
            if let Some(ref cc) = act.connector_config {
                let is_mutating = cc.operation.as_deref()
                    .map(|op| matches!(op, "post" | "put" | "delete" | "insert" | "update"))
                    .unwrap_or(false);
                if is_mutating {
                    result.infos.push(LintIssue {
                        code: "VIL-L009".into(),
                        message: "mutating connector without compensation (saga rollback not possible)".into(),
                        location: Some(loc.clone()),
                    });
                }
            }
        }
    }

    // VIL-L006: Dangling edge
    for flow in flows {
        let all_ids: Vec<&str> = node_ids.iter().chain(control_ids.iter()).copied().collect();
        if !all_ids.contains(&flow.from.node.as_str()) && flow.from.node != "end" {
            result.errors.push(LintIssue {
                code: "VIL-L006".into(),
                message: format!("flow '{}' references unknown from node '{}'", flow.id, flow.from.node),
                location: Some(format!("flow.{}", flow.id)),
            });
        }
        if !all_ids.contains(&flow.to.node.as_str()) && flow.to.node != "end" {
            result.errors.push(LintIssue {
                code: "VIL-L006".into(),
                message: format!("flow '{}' references unknown to node '{}'", flow.id, flow.to.node),
                location: Some(format!("flow.{}", flow.id)),
            });
        }
    }

    // VIL-L008: Durability not configured
    if doc.spec.durability.is_none() {
        result.infos.push(LintIssue {
            code: "VIL-L008".into(),
            message: "durability not configured (defaults to eventual)".into(),
            location: None,
        });
    }
}

/// Lint all VWFD files in directory.
pub fn lint_dir(dir: &str) -> Vec<LintResult> {
    let mut results = Vec::new();
    let path = Path::new(dir);
    if !path.is_dir() { return results; }

    if let Ok(entries) = std::fs::read_dir(path) {
        for entry in entries.flatten() {
            let ext = entry.path().extension()
                .and_then(|e| e.to_str()).unwrap_or("").to_string();
            if ext == "yaml" || ext == "yml" || ext == "vwfd" {
                results.push(lint_vwfd(&entry.path().display().to_string()));
            }
        }
    }
    results
}

// ── vil export --vwfd ──

/// Extract VWFD_YAML constants from Rust source files.
/// Scans for `pub const VWFD_YAML: &str =` patterns.
pub fn export_vwfd_from_source(src_dir: &str, output_dir: &str) -> Result<Vec<String>, String> {
    let mut exported = Vec::new();
    let src_path = Path::new(src_dir);

    if !src_path.is_dir() {
        return Err(format!("{} is not a directory", src_dir));
    }

    let _ = std::fs::create_dir_all(output_dir);

    fn scan_files(dir: &Path, output_dir: &str, exported: &mut Vec<String>) {
        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    scan_files(&path, output_dir, exported);
                } else if path.extension().and_then(|e| e.to_str()) == Some("rs") {
                    if let Ok(content) = std::fs::read_to_string(&path) {
                        // Find VWFD_YAML constants
                        for line in content.lines() {
                            if line.contains("pub const VWFD_YAML") {
                                // Extract the YAML between r#" and "#
                                if let Some(start) = content.find("VWFD_YAML: &str = r#\"") {
                                    let yaml_start = start + "VWFD_YAML: &str = r#\"".len();
                                    if let Some(end) = content[yaml_start..].find("\"#;") {
                                        let yaml = &content[yaml_start..yaml_start + end];
                                        // Try to get workflow id from YAML
                                        let id = yaml.lines()
                                            .find(|l| l.trim().starts_with("id:"))
                                            .and_then(|l| l.split(':').nth(1))
                                            .map(|s| s.trim().to_string())
                                            .unwrap_or_else(|| "unknown".into());

                                        let out_file = format!("{}/{}.yaml", output_dir, id);
                                        if std::fs::write(&out_file, yaml).is_ok() {
                                            exported.push(out_file);
                                        }
                                    }
                                }
                                break; // one per file
                            }
                        }
                    }
                }
            }
        }
    }

    scan_files(src_path, output_dir, &mut exported);
    Ok(exported)
}

#[cfg(test)]
mod tests {
    use super::*;

    const VALID_WF: &str = r#"
version: "3.0"
metadata:
  id: lint-test
spec:
  activities:
    - id: trigger
      activity_type: Trigger
      trigger_config:
        trigger_type: webhook
        webhook_config: { path: /test }
        response_mode: buffered
        end_activity: respond
      output_variable: trigger_payload
    - id: call_api
      activity_type: Connector
      connector_config:
        connector_ref: vastar.http
        operation: post
      input_mappings:
        - target: url
          source: { language: literal, source: "http://example.com" }
      output_variable: api_result
    - id: respond
      activity_type: EndTrigger
      end_trigger_config:
        trigger_ref: trigger
        final_response:
          language: vil-expr
          source: 'api_result'
    - id: end
      activity_type: End
  flows:
    - { id: f1, from: { node: trigger }, to: { node: call_api } }
    - { id: f2, from: { node: call_api }, to: { node: respond } }
    - { id: f3, from: { node: respond }, to: { node: end } }
"#;

    #[test]
    fn test_lint_valid() {
        let mut result = LintResult { file: "test".into(), errors: vec![], warnings: vec![], infos: vec![] };
        lint_yaml(VALID_WF, &mut result);
        assert!(result.errors.is_empty(), "errors: {:?}", result.errors);
    }

    #[test]
    fn test_lint_missing_retry() {
        let mut result = LintResult { file: "test".into(), errors: vec![], warnings: vec![], infos: vec![] };
        lint_yaml(VALID_WF, &mut result);
        // vastar.http without retry_policy → warning VIL-L001
        assert!(result.warnings.iter().any(|w| w.code == "VIL-L001"));
    }

    #[test]
    fn test_lint_missing_timeout() {
        let mut result = LintResult { file: "test".into(), errors: vec![], warnings: vec![], infos: vec![] };
        lint_yaml(VALID_WF, &mut result);
        assert!(result.warnings.iter().any(|w| w.code == "VIL-L004"));
    }

    #[test]
    fn test_lint_no_durability() {
        let mut result = LintResult { file: "test".into(), errors: vec![], warnings: vec![], infos: vec![] };
        lint_yaml(VALID_WF, &mut result);
        assert!(result.infos.iter().any(|i| i.code == "VIL-L008"));
    }

    #[test]
    fn test_lint_bad_connector_ref() {
        let yaml = r#"
version: "3.0"
metadata: { id: bad-ref }
spec:
  activities:
    - id: trigger
      activity_type: Trigger
      trigger_config: { trigger_type: webhook, webhook_config: { path: /t } }
    - id: bad
      activity_type: Connector
      connector_config:
        connector_ref: unknown.service
        operation: get
    - id: end
      activity_type: End
  flows:
    - { id: f1, from: { node: trigger }, to: { node: bad } }
    - { id: f2, from: { node: bad }, to: { node: end } }
"#;
        let mut result = LintResult { file: "test".into(), errors: vec![], warnings: vec![], infos: vec![] };
        lint_yaml(yaml, &mut result);
        assert!(result.errors.iter().any(|e| e.code == "VIL-L003"));
    }

    #[test]
    fn test_lint_dangling_edge() {
        let yaml = r#"
version: "3.0"
metadata: { id: dangling }
spec:
  activities:
    - id: trigger
      activity_type: Trigger
      trigger_config: { trigger_type: webhook, webhook_config: { path: /t } }
    - id: end
      activity_type: End
  flows:
    - { id: f1, from: { node: trigger }, to: { node: nonexistent } }
"#;
        let mut result = LintResult { file: "test".into(), errors: vec![], warnings: vec![], infos: vec![] };
        lint_yaml(yaml, &mut result);
        // Compiler catches dangling edge as compile error (VIL-L000) before lint rules run
        assert!(!result.errors.is_empty(), "should have errors for dangling edge");
        assert!(result.errors.iter().any(|e| e.message.contains("nonexistent")));
    }

    #[test]
    fn test_lint_mutating_no_compensation() {
        let mut result = LintResult { file: "test".into(), errors: vec![], warnings: vec![], infos: vec![] };
        lint_yaml(VALID_WF, &mut result);
        // post without compensation → info VIL-L009
        assert!(result.infos.iter().any(|i| i.code == "VIL-L009"));
    }
}
