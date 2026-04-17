//! vil provision prepare — extract .native() handlers from source, compile .so + .wasm
//!
//! Scans a project's src/main.rs for .native("name", handler) calls:
//!   - Inline closures: .native("name", |input| { ... })
//!   - Function references: .native("name", my_func) where my_func is defined above main()
//!
//! For each handler, generates a cdylib crate using vil_handler! macro,
//! then compiles all via a temp Cargo workspace → .so files.
//!
//! Also collects/compiles WASM modules from the project's wasm/ directory.

use std::path::{Path, PathBuf};
use colored::*;

/// Extracted handler from Rust source.
#[derive(Debug)]
struct ExtractedHandler {
    /// Handler name as registered: .native("this_name", ...)
    name: String,
    /// The complete code for lib.rs: helper functions + vil_handler! invocation
    lib_rs_code: String,
}

// ═══════════════════════════════════════════════════════════════════
// Public entry point
// ═══════════════════════════════════════════════════════════════════

pub fn run_prepare(
    path: &str,
    plugin_dir: &str,
    wasm_dir: &str,
    build_dir: &str,
    so_only: bool,
    wasm_only: bool,
    clean: bool,
    dry_run: bool,
    jobs: Option<usize>,
) -> Result<(), String> {
    let source_path = resolve_source_path(path)?;
    let project_dir = source_path.parent().and_then(|p| p.parent())
        .unwrap_or_else(|| Path::new(path));

    println!("  {} {}", "Source:".cyan().bold(), source_path.display());

    if !wasm_only {
        // ── Phase 1: Extract NativeCode handlers ──
        println!("\n  {} Extracting .native() handlers...", "→".cyan());
        let source = std::fs::read_to_string(&source_path)
            .map_err(|e| format!("read {}: {}", source_path.display(), e))?;

        let handlers = extract_handlers(&source)?;
        if handlers.is_empty() {
            println!("    {} No .native() handlers found", "⊘".yellow());
        } else {
            println!("    Found {} handler(s):", handlers.len());
            for h in &handlers {
                println!("      {} {}", "•".dimmed(), h.name);
            }
        }

        if dry_run {
            println!("\n  {} Dry-run mode — not compiling", "⊘".yellow());
        } else if !handlers.is_empty() {
            // ── Phase 2: Generate Cargo workspace ──
            println!("\n  {} Generating Cargo workspace...", "→".cyan());
            let vil_sdk_path = find_vil_plugin_sdk(path)?;
            generate_workspace(build_dir, &handlers, &vil_sdk_path, clean)?;
            println!("    Workspace: {}", build_dir);

            // ── Phase 3: Compile ──
            println!("\n  {} Compiling {} handler(s)...", "→".cyan(), handlers.len());
            compile_workspace(build_dir, jobs)?;

            // ── Phase 4: Collect .so files (only for extracted handlers) ──
            let handler_names: Vec<&str> = handlers.iter().map(|h| h.name.as_str()).collect();
            println!("\n  {} Collecting .so files → {}", "→".cyan(), plugin_dir);
            let collected = collect_so_files(build_dir, plugin_dir, &handler_names)?;
            println!("    {} .so file(s) ready", collected.to_string().green().bold());
        }
    }

    // ── Phase 4b: Patch sidecar commands into workflow YAML ──
    if !wasm_only && !dry_run {
        if let Ok(src) = std::fs::read_to_string(&source_path) {
            let vil_root = find_vil_workspace_root(path);

            // Existing .sidecar() calls — inject command into YAML
            let sidecar_cmds = extract_sidecar_commands(&src, vil_root.as_deref());
            if !sidecar_cmds.is_empty() {
                let workflows_dir = find_workflows_dir(project_dir);
                if let Some(wf_dir) = &workflows_dir {
                    println!("\n  {} Patching {} sidecar command(s) into workflow YAML", "→".cyan(), sidecar_cmds.len());
                    patch_sidecar_commands(wf_dir, &sidecar_cmds);
                }
            }

            // ── Phase 4c: Convert Python .wasm() → Sidecar in YAML ──
            let python_wasm = extract_python_wasm_calls(&src, vil_root.as_deref());
            if !python_wasm.is_empty() {
                let workflows_dir = find_workflows_dir(project_dir);
                if let Some(wf_dir) = &workflows_dir {
                    println!("\n  {} Converting {} Python .wasm() → Sidecar in workflow YAML", "→".cyan(), python_wasm.len());
                    for (module_ref, py_path) in &python_wasm {
                        println!("    {} {} → python3 {}", "⚠".yellow(), module_ref, py_path);
                    }
                    patch_python_wasm_to_sidecar(wf_dir, &python_wasm);
                }
            }

            // ── Phase 4d: Compile Java sidecar sources ──
            for (_, cmd) in &sidecar_cmds {
                compile_java_in_command(cmd);
            }
        }
    }

    if !so_only {
        // ── Phase 5: Collect/compile WASM ──
        println!("\n  {} Collecting WASM modules → {}", "→".cyan(), wasm_dir);
        let wasm_count = collect_wasm(project_dir, wasm_dir, dry_run)?;
        println!("    {} .wasm file(s) ready", wasm_count.to_string().green().bold());
    }

    println!("\n  {}", "Prepare complete.".green().bold());
    Ok(())
}

// ═══════════════════════════════════════════════════════════════════
// Source path resolution
// ═══════════════════════════════════════════════════════════════════

fn resolve_source_path(path: &str) -> Result<PathBuf, String> {
    let p = Path::new(path);
    // Direct .rs file
    if p.is_file() && p.extension().map_or(false, |e| e == "rs") {
        return Ok(p.to_path_buf());
    }
    // Project dir → try common locations
    if p.is_dir() {
        for candidate in &[
            "src/main.rs",
            "vwfd/src/main.rs",
            "src/lib.rs",
        ] {
            let c = p.join(candidate);
            if c.exists() { return Ok(c); }
        }
        return Err(format!(
            "No Rust source found in '{}'. Expected src/main.rs or vwfd/src/main.rs",
            path
        ));
    }
    Err(format!("'{}' is not a file or directory", path))
}

// ═══════════════════════════════════════════════════════════════════
// Top-level declaration extraction (static, const, struct, enum, type)
// ═══════════════════════════════════════════════════════════════════

fn extract_toplevel_declarations(preamble: &str) -> String {
    let mut result = String::new();
    let lines: Vec<&str> = preamble.lines().collect();
    let mut i = 0;

    while i < lines.len() {
        let trimmed = lines[i].trim();

        // Match: static, const, struct, enum, type (top-level only, not indented)
        let is_decl = trimmed.starts_with("static ")
            || trimmed.starts_with("const ")
            || trimmed.starts_with("pub static ")
            || trimmed.starts_with("pub const ")
            || trimmed.starts_with("struct ")
            || trimmed.starts_with("pub struct ")
            || trimmed.starts_with("enum ")
            || trimmed.starts_with("pub enum ")
            || trimmed.starts_with("type ")
            || trimmed.starts_with("pub type ");

        // Also match #[derive(...)] before struct/enum
        if trimmed.starts_with("#[derive(") || trimmed.starts_with("#[allow(") {
            // Check if next non-attr line is struct/enum
            let mut j = i + 1;
            while j < lines.len() {
                let next = lines[j].trim();
                if next.starts_with("#[") || next.is_empty() { j += 1; continue; }
                if next.starts_with("struct ") || next.starts_with("pub struct ")
                    || next.starts_with("enum ") || next.starts_with("pub enum ") {
                    // Collect from attribute through end of struct/enum
                    let start = i;
                    // Find closing brace or semicolon
                    let mut k = j;
                    if lines[k].contains('{') {
                        let mut depth = 0;
                        while k < lines.len() {
                            for ch in lines[k].chars() {
                                if ch == '{' { depth += 1; }
                                if ch == '}' { depth -= 1; }
                            }
                            if depth == 0 { break; }
                            k += 1;
                        }
                    }
                    for line_idx in start..=k.min(lines.len() - 1) {
                        result.push_str(lines[line_idx]);
                        result.push('\n');
                    }
                    i = k + 1;
                }
                break;
            }
            if i > j { continue; } // already advanced
        }

        if !is_decl { i += 1; continue; }

        // Single-line declaration (ends with ;)
        if trimmed.ends_with(';') {
            result.push_str(lines[i]);
            result.push('\n');
            i += 1;
            continue;
        }

        // Multi-line: collect until matching ; at depth 0
        // Handles: const X: &[T] = &[\n...\n];
        //          static X: Mutex<...> = Mutex::new(None);
        //          struct X {\n...\n}
        let mut depth_brace = 0i32;
        let mut depth_bracket = 0i32;
        let start = i;
        loop {
            let line = if i < lines.len() { lines[i] } else { break; };
            for ch in line.chars() {
                match ch {
                    '{' => depth_brace += 1,
                    '}' => depth_brace -= 1,
                    '[' => depth_bracket += 1,
                    ']' => depth_bracket -= 1,
                    _ => {}
                }
            }
            let at_end = line.trim().ends_with(';') || (depth_brace <= 0 && depth_bracket <= 0 && line.contains('}'));
            i += 1;
            if at_end && depth_brace <= 0 && depth_bracket <= 0 { break; }
            if i >= lines.len() { break; }
        }
        for line_idx in start..i.min(lines.len()) {
            result.push_str(lines[line_idx]);
            result.push('\n');
        }
    }

    result
}

// ═══════════════════════════════════════════════════════════════════
// Handler extraction — regex-based parsing of .native() calls
// ═══════════════════════════════════════════════════════════════════

fn extract_handlers(source: &str) -> Result<Vec<ExtractedHandler>, String> {
    // Split source into: preamble (before fn main) and main body
    let (preamble, _main_body) = split_at_main(source);

    // Collect all top-level function definitions from preamble
    let helper_functions = preamble.to_string();

    // Collect use statements from preamble (skip vil_vwfd, tokio, serde_json — already provided)
    let use_statements: String = source.lines()
        .filter(|l| {
            let trimmed = l.trim();
            trimmed.starts_with("use ") && !trimmed.contains("vil_vwfd")
                && !trimmed.contains("tokio")
                && !trimmed.contains("serde_json")
        })
        .map(|l| format!("{}\n", l))
        .collect();

    // Collect top-level declarations: static, const, struct, enum, type alias
    // Handles multi-line declarations (const ARRAY: &[...] = &[\n...\n];)
    let static_decls = extract_toplevel_declarations(&preamble);

    let mut handlers = Vec::new();

    // ── Find all .native("name", handler) calls ──
    let mut search_pos = 0;

    while let Some(native_pos) = source[search_pos..].find(".native(") {
        let abs_pos = search_pos + native_pos;
        let after_native = &source[abs_pos + 8..]; // skip ".native("

        // Extract handler name (first string argument)
        let name = match extract_string_literal(after_native) {
            Some(n) => n,
            None => { search_pos = abs_pos + 8; continue; }
        };

        // Find the comma after the name, then extract the handler expression
        let after_name = &after_native[name.len() + 2..]; // skip "name"
        let comma_pos = match after_name.find(',') {
            Some(p) => p,
            None => { search_pos = abs_pos + 8; continue; }
        };
        let after_comma = after_name[comma_pos + 1..].trim_start();

        // Determine if it's a closure or function reference
        let lib_rs = if after_comma.starts_with('|') {
            // Inline closure: extract the full closure body
            let closure = match extract_closure(after_comma) {
                Some(c) => c,
                None => { search_pos = abs_pos + 8; continue; }
            };
            format!(
                "#![allow(unused_imports, unused_variables, dead_code)]\n\
                 use vil_plugin_sdk::{{vil_handler, serde_json}};\n\
                 use serde_json::{{Value, json}};\n\
                 {}\n\
                 {}\n\
                 {}\n\
                 vil_handler!(\"{}\", {});\n",
                use_statements.trim(),
                static_decls.trim(),
                extract_referenced_helpers(&closure, &helper_functions),
                name,
                closure,
            )
        } else {
            // Function reference: extract the function name
            let func_name = after_comma.split(|c: char| c == ')' || c == '\n')
                .next().unwrap_or("").trim().trim_end_matches(')');
            if func_name.is_empty() { search_pos = abs_pos + 8; continue; }

            // Include the referenced function and any helpers it calls
            let func_body = extract_function_def(func_name, &helper_functions);
            format!(
                "#![allow(unused_imports, unused_variables, dead_code)]\n\
                 use vil_plugin_sdk::{{vil_handler, serde_json}};\n\
                 use serde_json::{{Value, json}};\n\
                 {}\n\
                 {}\n\
                 {}\n\
                 vil_handler!(\"{}\", {});\n",
                use_statements.trim(),
                static_decls.trim(),
                func_body,
                name,
                func_name,
            )
        };

        handlers.push(ExtractedHandler {
            name: name.clone(),
            lib_rs_code: lib_rs,
        });

        search_pos = abs_pos + 8;
    }

    Ok(handlers)
}

// ═══════════════════════════════════════════════════════════════════
// Sidecar command extraction + YAML patching
// ═══════════════════════════════════════════════════════════════════

/// Find VIL workspace root (directory containing Cargo.toml with [workspace]).
fn find_vil_workspace_root(path: &str) -> Option<String> {
    let mut dir = std::fs::canonicalize(path).ok()?;
    for _ in 0..10 {
        let cargo = dir.join("Cargo.toml");
        if cargo.exists() {
            if let Ok(content) = std::fs::read_to_string(&cargo) {
                if content.contains("[workspace]") {
                    return Some(dir.to_string_lossy().to_string());
                }
            }
        }
        if !dir.pop() { break; }
    }
    None
}

/// Extract .sidecar("name", "command") mappings from main.rs.
/// Resolves relative paths in commands to absolute paths.
fn extract_sidecar_commands(source: &str, vil_root: Option<&str>) -> Vec<(String, String)> {
    let mut commands = Vec::new();
    let mut pos = 0;
    while let Some(idx) = source[pos..].find(".sidecar(") {
        let abs = pos + idx + 9;
        let after = &source[abs..];
        if let Some(name) = extract_string_literal(after) {
            let after_name = &after[name.len() + 2..];
            if let Some(comma) = after_name.find(',') {
                let after_comma = after_name[comma + 1..].trim_start();
                if let Some(command) = extract_string_literal(after_comma) {
                    // Resolve relative file paths in command to absolute
                    let resolved = resolve_command_paths(&command, vil_root);
                    commands.push((name, resolved));
                }
            }
        }
        pos = abs;
    }
    commands
}

/// Resolve relative file paths in sidecar commands to absolute paths.
/// e.g. "python3 examples/foo/bar.py" → "python3 /home/.../vil/examples/foo/bar.py"
fn resolve_command_paths(command: &str, vil_root: Option<&str>) -> String {
    let parts: Vec<&str> = command.splitn(2, ' ').collect();
    if parts.len() < 2 { return command.to_string(); }
    let (program, rest) = (parts[0], parts[1]);

    // Force unbuffered output for Python sidecars
    let program = if program == "python3" || program == "python" {
        "python3 -u"
    } else {
        program
    };

    let mut resolved_parts: Vec<String> = vec![program.to_string()];
    for part in rest.split_whitespace() {
        if part.starts_with('-') {
            resolved_parts.push(part.to_string());
            continue;
        }
        // Try resolve relative to VIL workspace root first
        if let Some(root) = vil_root {
            let abs_path = Path::new(root).join(part);
            if abs_path.exists() {
                resolved_parts.push(abs_path.to_string_lossy().to_string());
                continue;
            }
        }
        // Try resolve from CWD
        let p = Path::new(part);
        if p.exists() {
            if let Ok(abs) = std::fs::canonicalize(p) {
                resolved_parts.push(abs.to_string_lossy().to_string());
                continue;
            }
        }
        resolved_parts.push(part.to_string());
    }
    resolved_parts.join(" ")
}

/// Extract .wasm("name", "path/to/file.py") calls — Python files that need sidecar conversion.
/// Returns Vec<(module_ref, absolute_py_path)>.
fn extract_python_wasm_calls(source: &str, vil_root: Option<&str>) -> Vec<(String, String)> {
    let mut results = Vec::new();
    let mut pos = 0;
    while let Some(idx) = source[pos..].find(".wasm(") {
        let abs = pos + idx + 6;
        let after = &source[abs..];
        if let Some(name) = extract_string_literal(after) {
            let after_name = &after[name.len() + 2..];
            if let Some(comma) = after_name.find(',') {
                let after_comma = after_name[comma + 1..].trim_start();
                if let Some(file_path) = extract_string_literal(after_comma) {
                    if file_path.ends_with(".py") {
                        // Resolve to absolute path
                        let resolved = if let Some(root) = vil_root {
                            let abs_path = Path::new(root).join(&file_path);
                            if abs_path.exists() {
                                abs_path.to_string_lossy().to_string()
                            } else {
                                file_path.clone()
                            }
                        } else {
                            file_path.clone()
                        };
                        results.push((name, resolved));
                    }
                }
            }
        }
        pos = abs;
    }
    results
}

/// Patch workflow YAML: convert Python Function (WASM) activities to Sidecar.
/// Changes activity_type: Function + wasm_config → activity_type: Sidecar + sidecar_config.
fn patch_python_wasm_to_sidecar(wf_dir: &Path, python_wasm: &[(String, String)]) {
    let entries = match std::fs::read_dir(wf_dir) {
        Ok(e) => e,
        Err(_) => return,
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if !path.extension().map_or(false, |e| e == "yaml" || e == "yml") { continue; }
        let content = match std::fs::read_to_string(&path) {
            Ok(c) => c,
            Err(_) => continue,
        };

        let mut modified = false;
        let mut new_content = content.clone();

        for (module_ref, py_path) in python_wasm {
            if !new_content.contains(&format!("module_ref: {}", module_ref)) { continue; }

            // Replace activity_type: Function → Sidecar (line by line)
            let lines: Vec<&str> = new_content.lines().collect();
            let mut patched: Vec<String> = Vec::new();
            let mut i = 0;
            while i < lines.len() {
                let line = lines[i];
                let trimmed = line.trim();

                // Find Function activity that references this module_ref
                if trimmed.starts_with("activity_type: Function") {
                    // Look ahead to confirm it's our module
                    let mut is_match = false;
                    for j in (i+1)..lines.len().min(i+15) {
                        if lines[j].trim().contains(&format!("module_ref: {}", module_ref)) {
                            is_match = true;
                            break;
                        }
                        if lines[j].trim().starts_with("- id:") { break; }
                    }

                    if is_match {
                        let indent = &line[..line.len() - trimmed.len()];
                        patched.push(format!("{}activity_type: Sidecar", indent));
                        i += 1;

                        // Replace wasm_config block with sidecar_config
                        while i < lines.len() {
                            let t = lines[i].trim();
                            if t.starts_with("wasm_config:") {
                                let cfg_indent = &lines[i][..lines[i].len() - t.len()];
                                patched.push(format!(
                                    "{}sidecar_config: {{ target: {}, command: \"python3 -u {}\" }}",
                                    cfg_indent, module_ref, py_path
                                ));
                                i += 1;
                                // Skip remaining wasm_config sub-fields
                                while i < lines.len() {
                                    let next = lines[i].trim();
                                    if next.starts_with("module_ref:") || next.starts_with("function_name:")
                                        || next.starts_with("timeout_ms:") || next.starts_with("description:") {
                                        i += 1;
                                        continue;
                                    }
                                    break;
                                }
                                modified = true;
                                continue;
                            }
                            break;
                        }
                        continue;
                    }
                }

                patched.push(line.to_string());
                i += 1;
            }

            if modified {
                new_content = patched.join("\n");
            }
        }

        if modified {
            let filename = path.file_name().unwrap().to_string_lossy();
            if let Err(e) = std::fs::write(&path, &new_content) {
                println!("    {} patch {}: {}", "✗".red(), filename, e);
            } else {
                println!("    {} {} → Sidecar (Python)", "✓".green(), filename);
            }
        }
    }
}

/// Patch workflow YAML: add `command` to sidecar_config so server can spawn processes.
fn patch_sidecar_commands(wf_dir: &Path, sidecar_cmds: &[(String, String)]) {
    let entries = match std::fs::read_dir(wf_dir) {
        Ok(e) => e,
        Err(_) => return,
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if !path.extension().map_or(false, |e| e == "yaml" || e == "yml") { continue; }
        let content = match std::fs::read_to_string(&path) {
            Ok(c) => c,
            Err(_) => continue,
        };

        let mut modified = false;
        let mut new_lines: Vec<String> = Vec::new();

        for line in content.lines() {
            new_lines.push(line.to_string());
            // After sidecar_config with target, inject command if missing
            let trimmed = line.trim();
            if trimmed.starts_with("sidecar_config:") {
                for (name, command) in sidecar_cmds {
                    if trimmed.contains(&format!("target: {}", name)) && !trimmed.contains("command:") {
                        // Inline format: sidecar_config: { target: X }
                        // Replace last line to add command
                        let last = new_lines.last_mut().unwrap();
                        *last = last.replace(
                            &format!("target: {}", name),
                            &format!("target: {}, command: \"{}\"", name, command),
                        );
                        modified = true;
                    }
                }
            }
        }

        if modified {
            let filename = path.file_name().unwrap().to_string_lossy();
            let joined = new_lines.join("\n");
            if let Err(e) = std::fs::write(&path, &joined) {
                println!("    {} patch {}: {}", "✗".red(), filename, e);
            } else {
                println!("    {} patched sidecar command in {}", "✓".green(), filename);
            }
        }
    }
}

/// Split source into (preamble before fn main, main body).
/// Only matches `fn main()` at the start of a line (not inside string literals).
fn split_at_main(source: &str) -> (&str, &str) {
    // Search for `fn main()` or `async fn main()` at line start
    for pattern in &["\nasync fn main()", "\nfn main()"] {
        if let Some(pos) = source.find(pattern) {
            return (&source[..pos], &source[pos + 1..]);
        }
    }
    // Fallback: check if source starts with fn main
    if source.starts_with("fn main()") || source.starts_with("async fn main()") {
        return ("", source);
    }
    (source, "")
}

/// Extract a string literal starting at position: "name" → name
fn extract_string_literal(s: &str) -> Option<String> {
    let s = s.trim_start();
    if !s.starts_with('"') { return None; }
    let end = s[1..].find('"')?;
    Some(s[1..=end].to_string())
}

/// Extract a closure expression starting with |...|, handling nested braces.
fn extract_closure(s: &str) -> Option<String> {
    // s starts with |
    let pipe_end = s[1..].find('|')? + 2; // position after closing |
    let after_params = s[pipe_end..].trim_start();

    if after_params.starts_with('{') {
        // Block closure: find matching }
        let block_start = s.len() - after_params.len();
        let block_end = find_matching_brace(s, block_start)?;
        Some(s[..block_end + 1].to_string())
    } else {
        // Expression closure (single expression until `)` or `,`)
        // Find the closing ) of .native() call
        let end = find_native_close(s)?;
        Some(s[..end].trim().to_string())
    }
}

/// Find matching closing brace, respecting nesting and string literals.
fn find_matching_brace(s: &str, start: usize) -> Option<usize> {
    let bytes = s.as_bytes();
    let mut depth = 0;
    let mut i = start;
    let mut in_string = false;
    let mut in_char = false;

    while i < bytes.len() {
        let b = bytes[i];
        if in_string {
            if b == b'\\' { i += 1; } // skip escaped char
            else if b == b'"' { in_string = false; }
        } else if in_char {
            if b == b'\\' { i += 1; }
            else if b == b'\'' { in_char = false; }
        } else {
            match b {
                b'"' => in_string = true,
                b'\'' => in_char = true,
                b'{' => depth += 1,
                b'}' => {
                    depth -= 1;
                    if depth == 0 { return Some(i); }
                }
                _ => {}
            }
        }
        i += 1;
    }
    None
}

/// Find the closing ) of a .native() call from within the handler argument.
fn find_native_close(s: &str) -> Option<usize> {
    let mut depth_paren = 0i32;
    let mut depth_brace = 0i32;
    let mut in_string = false;
    let bytes = s.as_bytes();
    let mut i = 0;

    while i < bytes.len() {
        let b = bytes[i];
        if in_string {
            if b == b'\\' { i += 1; }
            else if b == b'"' { in_string = false; }
        } else {
            match b {
                b'"' => in_string = true,
                b'(' => depth_paren += 1,
                b')' => {
                    if depth_paren == 0 && depth_brace == 0 {
                        return Some(i);
                    }
                    depth_paren -= 1;
                }
                b'{' => depth_brace += 1,
                b'}' => depth_brace -= 1,
                _ => {}
            }
        }
        i += 1;
    }
    None
}

/// Extract a top-level function definition by name from source.
/// Also recursively includes any helper functions it calls.
fn extract_function_def(func_name: &str, source: &str) -> String {
    let mut result = String::new();
    let mut included: std::collections::HashSet<String> = std::collections::HashSet::new();
    collect_function_and_deps(func_name, source, &mut result, &mut included);
    result
}

fn collect_function_and_deps(
    func_name: &str,
    source: &str,
    result: &mut String,
    included: &mut std::collections::HashSet<String>,
) {
    if included.contains(func_name) { return; }
    included.insert(func_name.to_string());

    // Find `fn func_name(` or `fn func_name<` (generic) in source
    let pattern1 = format!("fn {}(", func_name);
    let pattern2 = format!("fn {}<", func_name);
    let pos_opt = source.find(&pattern1).or_else(|| source.find(&pattern2));
    if let Some(pos) = pos_opt {
        // Walk back to find any attributes or doc comments
        let start = walk_back_to_fn_start(source, pos);
        // Find the closing brace of the function body
        if let Some(brace_start) = source[pos..].find('{') {
            let abs_brace = pos + brace_start;
            if let Some(brace_end) = find_matching_brace(source, abs_brace) {
                let func_code = &source[start..=brace_end];

                // Find helper functions called within this function
                let helper_names = find_called_functions(func_code, source);
                for helper in &helper_names {
                    collect_function_and_deps(helper, source, result, included);
                }

                result.push_str(func_code);
                result.push('\n');
            }
        }
    }
}

/// Walk back from a `fn` keyword to include doc comments / attributes.
fn walk_back_to_fn_start(source: &str, fn_pos: usize) -> usize {
    let before = &source[..fn_pos];
    let mut start = fn_pos;
    for line in before.lines().rev() {
        let trimmed = line.trim();
        if trimmed.starts_with("///") || trimmed.starts_with("#[") || trimmed.is_empty() {
            start -= line.len() + 1; // +1 for newline
        } else {
            break;
        }
    }
    start.max(0)
}

/// Find function names called within code that are defined in source (top-level fns).
fn find_called_functions(code: &str, source: &str) -> Vec<String> {
    // Collect all top-level function names from source
    let mut all_fns = Vec::new();
    let mut search = 0;
    while let Some(pos) = source[search..].find("\nfn ") {
        let abs = search + pos + 4;
        if let Some(paren) = source[abs..].find('(') {
            let raw_name = source[abs..abs + paren].trim();
            // Strip generic params: "block_async<F: Future>" → "block_async"
            let name = raw_name.split('<').next().unwrap_or(raw_name).trim();
            if name != "main" && !name.is_empty() {
                all_fns.push(name.to_string());
            }
        }
        search = abs;
    }

    // Check which of these are called in code
    all_fns.into_iter()
        .filter(|name| {
            // Look for `name(` pattern in code, but not as part of `fn name(`
            let call_pattern = format!("{}(", name);
            let def_pattern = format!("fn {}(", name);
            code.contains(&call_pattern) && !code.contains(&def_pattern)
        })
        .collect()
}

/// Extract helper functions referenced by a closure body.
fn extract_referenced_helpers(closure: &str, helper_source: &str) -> String {
    let helpers = find_called_functions(closure, helper_source);
    let mut result = String::new();
    let mut included = std::collections::HashSet::new();
    for h in &helpers {
        collect_function_and_deps(h, helper_source, &mut result, &mut included);
    }
    result
}

// ═══════════════════════════════════════════════════════════════════
// Cargo workspace generation
// ═══════════════════════════════════════════════════════════════════

fn find_vil_plugin_sdk(project_path: &str) -> Result<String, String> {
    // Walk up from project_path to find the VIL workspace root (has crates/vil_plugin_sdk)
    let mut dir = std::fs::canonicalize(project_path)
        .map_err(|e| format!("canonicalize '{}': {}", project_path, e))?;

    for _ in 0..10 {
        let sdk_path = dir.join("crates").join("vil_plugin_sdk");
        if sdk_path.is_dir() {
            return Ok(sdk_path.to_string_lossy().to_string());
        }
        if !dir.pop() { break; }
    }

    // Fallback: check common locations
    let home = std::env::var("HOME").unwrap_or_default();
    let common = Path::new(&home).join("Prdmid/vil-project/vil/crates/vil_plugin_sdk");
    if common.is_dir() {
        return Ok(common.to_string_lossy().to_string());
    }

    Err("Cannot find vil_plugin_sdk crate. Ensure you're in a VIL workspace.".into())
}

fn generate_workspace(
    build_dir: &str,
    handlers: &[ExtractedHandler],
    vil_sdk_path: &str,
    clean: bool,
) -> Result<(), String> {
    let build = Path::new(build_dir);

    if clean && build.exists() {
        std::fs::remove_dir_all(build)
            .map_err(|e| format!("clean {}: {}", build_dir, e))?;
    }

    let crates_dir = build.join("crates");
    std::fs::create_dir_all(&crates_dir)
        .map_err(|e| format!("mkdir {}: {}", crates_dir.display(), e))?;

    // Generate workspace Cargo.toml
    let mut members = Vec::new();
    for h in handlers {
        let crate_name = format!("handler_{}", h.name);
        members.push(format!("\"crates/{}\"", crate_name));

        let crate_dir = crates_dir.join(&crate_name);
        let src_dir = crate_dir.join("src");
        std::fs::create_dir_all(&src_dir)
            .map_err(|e| format!("mkdir {}: {}", src_dir.display(), e))?;

        // Crate Cargo.toml — auto-detect dependencies from handler code
        let code = &h.lib_rs_code;
        let vil_crates_dir = Path::new(&vil_sdk_path).parent().unwrap_or(Path::new("."));
        let mut extra_deps = String::new();

        // VIL crates (path deps — sibling of vil_plugin_sdk)
        let vil_dep_map = [
            ("vil_orm::",        "vil_orm"),
            ("vil_db_sqlx::",    "vil_db_sqlx"),
            ("vil_db_redis::",   "vil_db_redis"),
            ("vil_expr::",       "vil_expr"),
            ("vil_trigger::",    "vil_trigger"),
            ("vil_server_core::", "vil_server_core"),
            ("vil_server_db::",  "vil_server_db"),
            ("vil_new_http::",   "vil_new_http"),
            ("vil_capsule::",    "vil_capsule"),
        ];
        for (pattern, crate_name) in &vil_dep_map {
            if code.contains(pattern) {
                let dep_path = vil_crates_dir.join(crate_name);
                if dep_path.is_dir() {
                    extra_deps += &format!("{} = {{ path = \"{}\" }}\n", crate_name, dep_path.display());
                }
            }
        }

        // External crates (registry deps)
        let ext_dep_map = [
            ("regex::",      "regex = \"1\""),
            ("sqlx::",       "sqlx = { version = \"0.8\", features = [\"runtime-tokio\", \"postgres\", \"sqlite\"] }"),
            ("tokio::",      "tokio = { version = \"1\", features = [\"full\"] }"),
            ("serde::",      "serde = { version = \"1\", features = [\"derive\"] }"),
            ("reqwest::",    "reqwest = { version = \"0.12\", features = [\"json\"] }"),
            ("chrono::",     "chrono = { version = \"0.4\", features = [\"serde\"] }"),
            ("uuid::",       "uuid = { version = \"1\", features = [\"v4\"] }"),
        ];
        for (pattern, dep_line) in &ext_dep_map {
            if code.contains(pattern) {
                extra_deps += dep_line;
                extra_deps += "\n";
            }
        }
        // Also detect derive macros: sqlx::FromRow, serde::Serialize/Deserialize
        if !extra_deps.contains("sqlx =") && (code.contains("sqlx::FromRow") || code.contains("sqlx::Type")) {
            extra_deps += "sqlx = { version = \"0.8\", features = [\"runtime-tokio\", \"postgres\", \"sqlite\"] }\n";
        }
        if !extra_deps.contains("serde =") && (code.contains("serde::Serialize") || code.contains("serde::Deserialize") || code.contains("Serialize, Deserialize") || code.contains("Serialize,")) {
            extra_deps += "serde = { version = \"1\", features = [\"derive\"] }\n";
        }

        let cargo_toml = format!(
            "[package]\n\
             name = \"{crate_name}\"\n\
             version = \"0.1.0\"\n\
             edition = \"2021\"\n\
             \n\
             [lib]\n\
             crate-type = [\"cdylib\"]\n\
             \n\
             [dependencies]\n\
             vil_plugin_sdk = {{ path = \"{sdk_path}\" }}\n\
             serde_json = \"1.0\"\n\
             {extra_deps}",
            crate_name = crate_name,
            sdk_path = vil_sdk_path,
            extra_deps = extra_deps,
        );
        std::fs::write(crate_dir.join("Cargo.toml"), &cargo_toml)
            .map_err(|e| format!("write Cargo.toml: {}", e))?;

        // src/lib.rs
        std::fs::write(src_dir.join("lib.rs"), &h.lib_rs_code)
            .map_err(|e| format!("write lib.rs: {}", e))?;
    }

    // Workspace Cargo.toml
    let workspace_toml = format!(
        "[workspace]\n\
         resolver = \"2\"\n\
         members = [{}]\n",
        members.join(", ")
    );
    std::fs::write(build.join("Cargo.toml"), &workspace_toml)
        .map_err(|e| format!("write workspace Cargo.toml: {}", e))?;

    Ok(())
}

fn compile_workspace(build_dir: &str, jobs: Option<usize>) -> Result<(), String> {
    let mut cmd = std::process::Command::new("cargo");
    cmd.arg("build").arg("--release");
    if let Some(j) = jobs {
        cmd.arg("-j").arg(j.to_string());
    }
    cmd.current_dir(build_dir);

    let status = cmd.status()
        .map_err(|e| format!("cargo build: {}", e))?;
    if !status.success() {
        return Err(format!(
            "cargo build failed (exit {}). Check {}/crates/ for errors.",
            status.code().unwrap_or(-1), build_dir
        ));
    }
    Ok(())
}

fn collect_so_files(build_dir: &str, plugin_dir: &str, handler_names: &[&str]) -> Result<u32, String> {
    let release_dir = Path::new(build_dir).join("target").join("release");
    let out_dir = Path::new(plugin_dir);
    std::fs::create_dir_all(out_dir)
        .map_err(|e| format!("mkdir {}: {}", plugin_dir, e))?;

    let mut count = 0u32;
    for name in handler_names {
        // Crate name: handler_{name} → .so file: libhandler_{name}.so
        let so_filename = format!("libhandler_{}.so", name);
        let so_path = release_dir.join(&so_filename);
        if so_path.exists() {
            let dest = out_dir.join(format!("{}.so", name));
            std::fs::copy(&so_path, &dest)
                .map_err(|e| format!("copy {} → {}: {}", so_path.display(), dest.display(), e))?;
            let size = std::fs::metadata(&dest).map(|m| m.len()).unwrap_or(0);
            println!("    {} {}.so ({})",
                "✓".green(),
                name,
                format_size(size).dimmed());
            count += 1;
        } else {
            println!("    {} {}.so (not found in build output)", "✗".red(), name);
        }
    }
    Ok(count)
}

// ═══════════════════════════════════════════════════════════════════
// WASM collection
// ═══════════════════════════════════════════════════════════════════

fn collect_wasm(project_dir: &Path, wasm_dir: &str, dry_run: bool) -> Result<u32, String> {
    let out_dir = Path::new(wasm_dir);
    if !dry_run {
        std::fs::create_dir_all(out_dir)
            .map_err(|e| format!("mkdir {}: {}", wasm_dir, e))?;
    }

    let mut count = 0u32;

    // Extract module_ref → file mapping from main.rs
    // Pattern: .wasm("module_ref", "path/to/file.wasm")
    let source_path = resolve_source_path(&project_dir.to_string_lossy());
    let wasm_mappings: Vec<(String, String)> = if let Ok(sp) = &source_path {
        if let Ok(source) = std::fs::read_to_string(sp) {
            extract_wasm_mappings(&source)
        } else { Vec::new() }
    } else { Vec::new() };

    // Look for wasm/ subdirectory in project
    let wasm_src_dirs = [
        project_dir.join("wasm"),
        project_dir.join("vwfd").join("wasm"),
    ];

    for wasm_src in &wasm_src_dirs {
        if !wasm_src.is_dir() { continue; }
        collect_wasm_recursive(wasm_src, out_dir, dry_run, &mut count, &wasm_mappings)?;
    }

    Ok(count)
}

/// Extract .wasm("module_ref", "path") mappings from main.rs source.
fn extract_wasm_mappings(source: &str) -> Vec<(String, String)> {
    let mut mappings = Vec::new();
    let mut pos = 0;
    while let Some(idx) = source[pos..].find(".wasm(") {
        let abs = pos + idx + 6;
        let after = &source[abs..];
        // Extract first string: module_ref
        if let Some(module_ref) = extract_string_literal(after) {
            // Find second string: file path
            let after_ref = &after[module_ref.len() + 2..];
            if let Some(comma) = after_ref.find(',') {
                let after_comma = after_ref[comma + 1..].trim_start();
                if let Some(file_path) = extract_string_literal(after_comma) {
                    // Extract just the filename stem from the path
                    let filename = std::path::Path::new(&file_path)
                        .file_stem()
                        .unwrap_or_default()
                        .to_string_lossy()
                        .to_string();
                    mappings.push((module_ref, filename));
                }
            }
        }
        pos = abs;
    }
    mappings
}

fn collect_wasm_recursive(
    dir: &Path,
    out_dir: &Path,
    dry_run: bool,
    count: &mut u32,
    wasm_mappings: &[(String, String)],
) -> Result<(), String> {
    let entries = std::fs::read_dir(dir)
        .map_err(|e| format!("read {}: {}", dir.display(), e))?;

    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_wasm_recursive(&path, out_dir, dry_run, count, wasm_mappings)?;
            continue;
        }

        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
        let stem = path.file_stem().unwrap_or_default().to_string_lossy().to_string();

        match ext {
            "wasm" => {
                // Pre-compiled — use module_ref name from mapping if available
                let output_name = wasm_mappings.iter()
                    .find(|(_, filename)| filename == &stem)
                    .map(|(module_ref, _)| module_ref.clone())
                    .unwrap_or_else(|| stem.clone());
                if dry_run {
                    println!("    {} {} → {}.wasm (pre-compiled)", "•".dimmed(), stem, output_name);
                } else {
                    let dest = out_dir.join(format!("{}.wasm", output_name));
                    std::fs::copy(&path, &dest)
                        .map_err(|e| format!("copy {}: {}", path.display(), e))?;
                    if output_name != stem {
                        println!("    {} {}.wasm → {}.wasm (renamed)", "✓".green(), stem, output_name);
                    } else {
                        println!("    {} {}.wasm (copied)", "✓".green(), stem);
                    }
                }
                *count += 1;
            }
            "rs" => {
                // Rust WASM source — skip if pre-compiled .wasm already collected
                let output_name = wasm_mappings.iter()
                    .find(|(_, filename)| filename == &stem)
                    .map(|(module_ref, _)| module_ref.clone())
                    .unwrap_or_else(|| stem.clone());
                if out_dir.join(format!("{}.wasm", output_name)).exists() { continue; }
                if dry_run {
                    println!("    {} {} (Rust → compile to {}.wasm)", "•".dimmed(), stem, output_name);
                    *count += 1;
                } else {
                    let dest = out_dir.join(format!("{}.wasm", output_name));
                    // Try wasm32-wasip1 first (newer), fallback to wasm32-wasi
                    let targets = ["wasm32-wasip1", "wasm32-wasi"];
                    let mut compiled = false;
                    for target in &targets {
                        let status = std::process::Command::new("rustc")
                            .args(["--target", target, "--edition", "2021", "-C", "opt-level=2"])
                            .arg("-o").arg(&dest)
                            .arg(&path)
                            .stderr(std::process::Stdio::null())
                            .status();
                        if let Ok(s) = status {
                            if s.success() {
                                println!("    {} {}.wasm (Rust → {})", "✓".green(), output_name, target);
                                *count += 1;
                                compiled = true;
                                break;
                            }
                        }
                    }
                    if !compiled {
                        println!("    {} {}.rs (compile failed — install: rustup target add wasm32-wasip1)", "✗".red(), stem);
                    }
                }
            }
            "go" => {
                let output_name = wasm_mappings.iter()
                    .find(|(_, filename)| filename == &stem)
                    .map(|(module_ref, _)| module_ref.clone())
                    .unwrap_or_else(|| stem.clone());
                if out_dir.join(format!("{}.wasm", output_name)).exists() { continue; }
                if dry_run {
                    println!("    {} {} (Go → compile to {}.wasm)", "•".dimmed(), stem, output_name);
                    *count += 1;
                } else {
                    let dest = out_dir.join(format!("{}.wasm", output_name));
                    let status = std::process::Command::new("go")
                        .args(["build", "-o"])
                        .arg(&dest)
                        .arg(&path)
                        .env("GOOS", "wasip1")
                        .env("GOARCH", "wasm")
                        .status();
                    match status {
                        Ok(s) if s.success() => {
                            println!("    {} {}.wasm (Go → wasip1)", "✓".green(), output_name);
                            *count += 1;
                        }
                        Ok(_) => println!("    {} {}.go (go build failed)", "✗".red(), stem),
                        Err(_) => println!("    {} {}.go (go not found — install Go)", "✗".red(), stem),
                    }
                }
            }
            "c" => {
                let output_name = wasm_mappings.iter()
                    .find(|(_, filename)| filename == &stem)
                    .map(|(module_ref, _)| module_ref.clone())
                    .unwrap_or_else(|| stem.clone());
                if out_dir.join(format!("{}.wasm", output_name)).exists() { continue; }
                if dry_run {
                    println!("    {} {} (C → compile to {}.wasm)", "•".dimmed(), stem, output_name);
                    *count += 1;
                } else {
                    let dest = out_dir.join(format!("{}.wasm", output_name));
                    // Try wasi-sdk clang if available
                    let wasi_sdk = std::env::var("WASI_SDK_PATH")
                        .unwrap_or_else(|_| "/opt/wasi-sdk".to_string());
                    let wasi_clang = Path::new(&wasi_sdk).join("bin/clang");

                    let status = if wasi_clang.exists() {
                        // wasi-sdk — full WASI libc support
                        std::process::Command::new(&wasi_clang)
                            .args(["--target=wasm32-wasi", "-O2", "-Wl,--no-entry", "-Wl,--export-all", "-o"])
                            .arg(&dest).arg(&path)
                            .status()
                    } else {
                        // Regular clang — no stdlib (nostdlib)
                        std::process::Command::new("clang")
                            .args(["--target=wasm32-unknown-unknown", "-nostdlib", "-O2",
                                   "-Wl,--no-entry", "-Wl,--export-all", "-o"])
                            .arg(&dest).arg(&path)
                            .stderr(std::process::Stdio::null())
                            .status()
                    };
                    match status {
                        Ok(s) if s.success() => {
                            println!("    {} {}.wasm (C → clang)", "✓".green(), output_name);
                            *count += 1;
                        }
                        Ok(_) => println!("    {} {}.c (clang failed — needs wasi-sdk for stdlib)", "✗".red(), stem),
                        Err(_) => println!("    {} {}.c (clang not found)", "✗".red(), stem),
                    }
                }
            }
            "ts" => {
                let output_name = wasm_mappings.iter()
                    .find(|(_, filename)| filename == &stem)
                    .map(|(module_ref, _)| module_ref.clone())
                    .unwrap_or_else(|| stem.clone());
                if out_dir.join(format!("{}.wasm", output_name)).exists() { continue; }
                if dry_run {
                    println!("    {} {} (TypeScript → compile to {}.wasm)", "•".dimmed(), stem, output_name);
                    *count += 1;
                } else {
                    let dest = out_dir.join(format!("{}.wasm", output_name));
                    // Try `asc` directly, fallback to `npx asc`
                    let status = std::process::Command::new("asc")
                        .arg(&path)
                        .arg("--outFile").arg(&dest)
                        .arg("--optimize")
                        .status()
                        .or_else(|_| {
                            std::process::Command::new("npx")
                                .args(["asc"]).arg(&path)
                                .arg("--outFile").arg(&dest)
                                .arg("--optimize")
                                .status()
                        });
                    match status {
                        Ok(s) if s.success() => {
                            println!("    {} {}.wasm (TypeScript → asc)", "✓".green(), output_name);
                            *count += 1;
                        }
                        Ok(_) => println!("    {} {}.ts (asc build failed)", "✗".red(), stem),
                        Err(_) => println!("    {} {}.ts (asc not found — npm i -g assemblyscript)", "✗".red(), stem),
                    }
                }
            }
            "py" => {
                // Python — cannot compile to WASM, suggest sidecar
                if dry_run {
                    println!("    {} {} (Python — use as sidecar, not WASM)", "⊘".yellow(), stem);
                } else {
                    println!("    {} {}.py (Python cannot compile to WASM — use .sidecar() instead)", "⊘".yellow(), stem);
                }
            }
            "java" => {
                // Java — only pre-compiled .wasm supported
                if dry_run {
                    println!("    {} {} (Java — pre-compiled .wasm only)", "⊘".yellow(), stem);
                } else {
                    println!("    {} {}.java (needs pre-compiled .wasm — use TeaVM or JWebAssembly)", "⊘".yellow(), stem);
                }
            }
            _ => {}
        }
    }
    Ok(())
}

// ═══════════════════════════════════════════════════════════════════
// Utilities
// ═══════════════════════════════════════════════════════════════════

// ═══════════════════════════════════════════════════════════════════
// Workflow YAML patching — rewrite Function/Sidecar → NativeCode
// ═══════════════════════════════════════════════════════════════════

/// If sidecar command is `java -cp <dir> <ClassName>`, compile .java → .class
fn compile_java_in_command(command: &str) {
    let parts: Vec<&str> = command.split_whitespace().collect();
    if parts.first() != Some(&"java") { return; }
    if let Some(cp_idx) = parts.iter().position(|&p| p == "-cp") {
        if cp_idx + 2 < parts.len() {
            let classpath = parts[cp_idx + 1];
            let classname = parts[cp_idx + 2];
            let java_file = Path::new(classpath).join(format!("{}.java", classname));
            let class_file = Path::new(classpath).join(format!("{}.class", classname));
            if java_file.exists() && !class_file.exists() {
                println!("    {} Compiling {}.java", "→".cyan(), classname);
                let status = std::process::Command::new("javac")
                    .arg(&java_file)
                    .status();
                match status {
                    Ok(s) if s.success() => println!("    {} {}.class", "✓".green(), classname),
                    _ => println!("    {} {}.java (javac failed)", "✗".red(), classname),
                }
            }
        }
    }
}

fn find_workflows_dir(project_dir: &Path) -> Option<PathBuf> {
    for candidate in &["workflows", "vwfd/workflows"] {
        let p = project_dir.join(candidate);
        if p.is_dir() { return Some(p); }
    }
    None
}

fn format_size(bytes: u64) -> String {
    if bytes > 1_048_576 {
        format!("{:.1} MB", bytes as f64 / 1_048_576.0)
    } else if bytes > 1024 {
        format!("{:.1} KB", bytes as f64 / 1024.0)
    } else {
        format!("{} B", bytes)
    }
}
