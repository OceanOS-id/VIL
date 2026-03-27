//! vil check — comprehensive manifest validation.
//!
//! `vil check <manifest.yaml>`
//!
//! Performs 9 validation checks beyond basic YAML parsing.

use crate::manifest::WorkflowManifest;
use colored::*;
use std::path::Path;

/// Run all checks on a manifest file.
pub fn run_check(manifest_path: &str) -> Result<(), String> {
    let base_dir = Path::new(manifest_path).parent().unwrap_or(Path::new("."));
    let mut passed = 0u32;
    let mut failed = 0u32;
    let mut warnings = 0u32;

    println!("{} Checking: {}\n", ">>>".cyan().bold(), manifest_path);

    // 1. Manifest parsing
    let manifest = match WorkflowManifest::from_file(manifest_path) {
        Ok(m) => {
            check_pass(&mut passed, "Manifest parsing");
            m
        }
        Err(e) => {
            check_fail(&mut failed, "Manifest parsing", &e);
            return report(passed, failed, warnings);
        }
    };

    // 2. Basic validation
    match manifest.validate() {
        Ok(()) => check_pass(&mut passed, "Field validation"),
        Err(errors) => {
            for e in &errors {
                check_fail(&mut failed, "Field validation", e);
            }
        }
    }

    // 3. Handler file existence
    let mut handler_ok = true;
    for (name, node) in &manifest.nodes {
        if let Some(code) = &node.code {
            if code.mode == "handler" {
                if let Some(handler) = &code.handler {
                    let handler_path = base_dir.join(format!("src/handlers/{}.rs", handler));
                    if !handler_path.exists() {
                        check_warn(&mut warnings, &format!(
                            "Handler file missing: {} (node '{}'). Generate with: vil generate handler {} --from {}",
                            handler_path.display(), name, handler, manifest_path
                        ));
                        handler_ok = false;
                    }
                }
            }
        }
    }
    for (_wf_name, wf) in &manifest.workflows {
        for task in &wf.tasks {
            if let Some(code) = &task.code {
                if code.mode == "handler" {
                    if let Some(handler) = &code.handler {
                        let handler_path = base_dir.join(format!("src/handlers/{}.rs", handler));
                        if !handler_path.exists() {
                            check_warn(&mut warnings, &format!(
                                "Handler file missing: {} (task '{}')",
                                handler_path.display(), task.id
                            ));
                            handler_ok = false;
                        }
                    }
                }
            }
        }
    }
    if handler_ok { check_pass(&mut passed, "Handler file existence"); }

    // 4. Script file existence
    let mut script_ok = true;
    for (name, node) in &manifest.nodes {
        if let Some(code) = &node.code {
            if code.mode == "script" {
                if let Some(src) = &code.source {
                    let script_path = base_dir.join(src);
                    if !script_path.exists() {
                        check_warn(&mut warnings, &format!(
                            "Script file missing: {} (node '{}')",
                            script_path.display(), name
                        ));
                        script_ok = false;
                    }
                }
            }
        }
    }
    if script_ok { check_pass(&mut passed, "Script file existence"); }

    // 5. WASM module existence
    let mut wasm_ok = true;
    for module in &manifest.vil_wasm {
        if let Some(wasm_path) = &module.wasm_path {
            let full_path = base_dir.join(wasm_path);
            if !full_path.exists() {
                check_warn(&mut warnings, &format!(
                    "WASM file missing: {} (module '{}')",
                    full_path.display(), module.name
                ));
                wasm_ok = false;
            }
        } else if let Some(source_dir) = &module.source_dir {
            let full_path = base_dir.join(source_dir);
            if !full_path.exists() {
                check_warn(&mut warnings, &format!(
                    "WASM source dir missing: {} (module '{}'). Run: vil wasm scaffold {} --language {}",
                    full_path.display(), module.name, module.name, module.language
                ));
                wasm_ok = false;
            }
        }
    }
    if wasm_ok { check_pass(&mut passed, "WASM module existence"); }

    // 6. Route DAG acyclicity
    // Note: bidirectional edges (A→B + B→A) are NORMAL in pipelines (request-response).
    // Only flag true cycles of length 3+ (A→B→C→A).
    {
        let mut adj: std::collections::HashMap<String, std::collections::HashSet<String>> = std::collections::HashMap::new();
        for route in &manifest.workflow_routes {
            let from = route.from.split('.').next().unwrap_or("").to_string();
            let to = route.to.split('.').next().unwrap_or("").to_string();
            if from != to { // skip self-loops
                adj.entry(from).or_default().insert(to);
            }
        }

        // Remove bidirectional pairs (A↔B is request-response, not a cycle)
        let keys: Vec<String> = adj.keys().cloned().collect();
        for a in &keys {
            if let Some(neighbors) = adj.get(a).cloned() {
                for b in &neighbors {
                    if let Some(b_neighbors) = adj.get_mut(b) {
                        b_neighbors.remove(a); // Remove B→A if A→B exists
                    }
                }
            }
        }

        // Now check for cycles in the cleaned DAG (length 3+)
        let mut has_cycle = false;
        let mut visited = std::collections::HashSet::new();
        let mut in_stack = std::collections::HashSet::new();

        fn dfs_cycle(
            node: &str,
            adj: &std::collections::HashMap<String, std::collections::HashSet<String>>,
            visited: &mut std::collections::HashSet<String>,
            in_stack: &mut std::collections::HashSet<String>,
        ) -> bool {
            visited.insert(node.to_string());
            in_stack.insert(node.to_string());
            if let Some(neighbors) = adj.get(node) {
                for next in neighbors {
                    if !visited.contains(next) {
                        if dfs_cycle(next, adj, visited, in_stack) { return true; }
                    } else if in_stack.contains(next) {
                        return true;
                    }
                }
            }
            in_stack.remove(node);
            false
        }

        for node_name in manifest.nodes.keys() {
            if !visited.contains(node_name) {
                if dfs_cycle(node_name, &adj, &mut visited, &mut in_stack) {
                    has_cycle = true;
                    break;
                }
            }
        }

        if has_cycle {
            check_fail(&mut failed, "Route DAG acyclicity", "Cycle detected in topology routes (length 3+)");
        } else {
            check_pass(&mut passed, "Route DAG acyclicity");
        }
    }

    // 7. Workflow DAG acyclicity
    {
        let mut wf_ok = true;
        for (wf_name, wf) in &manifest.workflows {
            let mut adj: std::collections::HashMap<String, Vec<String>> = std::collections::HashMap::new();
            for task in &wf.tasks {
                for dep in &task.deps {
                    adj.entry(dep.task_id().to_string()).or_default().push(task.id.clone());
                }
            }
            for branch in &wf.branches {
                for dep in &branch.deps {
                    adj.entry(dep.task_id().to_string()).or_default().push(branch.id.clone());
                }
            }

            // Simple cycle check
            let all_ids: Vec<String> = wf.tasks.iter().map(|t| t.id.clone())
                .chain(wf.branches.iter().map(|b| b.id.clone()))
                .collect();
            let mut visited = std::collections::HashSet::new();
            let mut in_stack = std::collections::HashSet::new();

            fn dfs_wf(
                node: &str,
                adj: &std::collections::HashMap<String, Vec<String>>,
                visited: &mut std::collections::HashSet<String>,
                in_stack: &mut std::collections::HashSet<String>,
            ) -> bool {
                visited.insert(node.to_string());
                in_stack.insert(node.to_string());
                if let Some(neighbors) = adj.get(node) {
                    for next in neighbors {
                        if !visited.contains(next) {
                            if dfs_wf(next, adj, visited, in_stack) { return true; }
                        } else if in_stack.contains(next) {
                            return true;
                        }
                    }
                }
                in_stack.remove(node);
                false
            }

            for id in &all_ids {
                if !visited.contains(id) {
                    if dfs_wf(id, &adj, &mut visited, &mut in_stack) {
                        check_fail(&mut failed, &format!("Workflow '{}' DAG acyclicity", wf_name),
                            "Cycle detected in task dependencies");
                        wf_ok = false;
                        break;
                    }
                }
            }
        }
        if wf_ok { check_pass(&mut passed, "Workflow DAG acyclicity"); }
    }

    // 8. Node type validity
    {
        let mut all_valid = true;
        for (name, node) in &manifest.nodes {
            match node.node_type.as_str() {
                "http-sink" | "http-source" | "transform" => {}
                other => {
                    if crate::node_types::find_node_type(other).is_none() {
                        check_warn(&mut warnings, &format!(
                            "Unknown node type '{}' on node '{}' (not in registry)",
                            other, name
                        ));
                        all_valid = false;
                    }
                }
            }
        }
        if all_valid { check_pass(&mut passed, "Node type validity"); }
    }

    // 9. Call target existence
    {
        let mut calls_ok = true;
        for (_wf_name, wf) in &manifest.workflows {
            for task in &wf.tasks {
                if let Some(call_path) = &task.call {
                    let full_path = base_dir.join(call_path);
                    if !full_path.exists() {
                        check_warn(&mut warnings, &format!(
                            "Call target missing: {} (task '{}')",
                            full_path.display(), task.id
                        ));
                        calls_ok = false;
                    }
                }
            }
            if let Some(on_complete) = &wf.on_complete {
                for path in [&on_complete.success, &on_complete.failure].into_iter().flatten() {
                    let full_path = base_dir.join(path);
                    if !full_path.exists() {
                        check_warn(&mut warnings, &format!(
                            "on_complete target missing: {}", full_path.display()
                        ));
                        calls_ok = false;
                    }
                }
            }
        }
        if calls_ok { check_pass(&mut passed, "Call target existence"); }
    }

    report(passed, failed, warnings)
}

fn check_pass(count: &mut u32, label: &str) {
    *count += 1;
    println!("  {} {}", "PASS".green().bold(), label);
}

fn check_fail(count: &mut u32, label: &str, detail: &str) {
    *count += 1;
    println!("  {} {} — {}", "FAIL".red().bold(), label, detail);
}

fn check_warn(count: &mut u32, detail: &str) {
    *count += 1;
    println!("  {} {}", "WARN".yellow().bold(), detail);
}

fn report(passed: u32, failed: u32, warnings: u32) -> Result<(), String> {
    println!("\n{} {} passed, {} failed, {} warnings",
        if failed == 0 { "RESULT".green().bold() } else { "RESULT".red().bold() },
        passed, failed, warnings
    );
    if failed > 0 {
        Err(format!("{} check(s) failed", failed))
    } else {
        Ok(())
    }
}
