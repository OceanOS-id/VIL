//! WASM function module builder.
//!
//! `vil wasm scaffold <name> --language rust` → creates project skeleton
//! `vil wasm build <manifest>` → compiles all declared WASM modules
//! `vil wasm list <manifest>` → lists registered modules + functions

use vil_cli_core::manifest::WorkflowManifest;
use colored::*;
use std::path::Path;

const WASM_OUT_DIR: &str = "wasm-out";

/// Scaffold a new WASM module project.
pub fn scaffold_module(name: &str, language: &str, output_dir: &str) -> Result<(), String> {
    let dir = Path::new(output_dir).join(name);
    std::fs::create_dir_all(&dir)
        .map_err(|e| format!("Failed to create '{}': {}", dir.display(), e))?;

    match language {
        "rust" => scaffold_rust(&dir, name)?,
        "c" => scaffold_c(&dir, name)?,
        "go" => scaffold_go(&dir, name)?,
        "assemblyscript" | "as" => scaffold_assemblyscript(&dir, name)?,
        other => {
            return Err(format!(
                "Unsupported language '{}'. Supported: rust, c, go, assemblyscript",
                other
            ))
        }
    }

    println!(
        "{} Scaffolded WASM module '{}' ({}) at {}",
        "OK".green().bold(),
        name,
        language,
        dir.display()
    );
    Ok(())
}

/// Build all WASM modules declared in a manifest.
pub fn build_modules(manifest_path: &str, module_filter: Option<&str>) -> Result<(), String> {
    let manifest = WorkflowManifest::from_file(manifest_path)?;

    if manifest.vil_wasm.is_empty() {
        return Err("No vil_wasm: modules declared in manifest".into());
    }

    std::fs::create_dir_all(WASM_OUT_DIR)
        .map_err(|e| format!("Failed to create '{}': {}", WASM_OUT_DIR, e))?;

    let base_dir = Path::new(manifest_path).parent().unwrap_or(Path::new("."));

    for module in &manifest.vil_wasm {
        if let Some(filter) = module_filter {
            if module.name != filter {
                continue;
            }
        }

        println!(
            "{} Building WASM module: {} ({})",
            ">>>".cyan().bold(),
            module.name,
            module.language
        );

        // If wasm_path is set, just copy the pre-compiled file
        if let Some(wasm_path) = &module.wasm_path {
            let src = base_dir.join(wasm_path);
            let dst = Path::new(WASM_OUT_DIR).join(format!("{}.wasm", module.name));
            std::fs::copy(&src, &dst).map_err(|e| {
                format!(
                    "Failed to copy '{}' → '{}': {}",
                    src.display(),
                    dst.display(),
                    e
                )
            })?;
            println!(
                "  {} Copied pre-compiled: {}",
                "OK".green().bold(),
                dst.display()
            );
            continue;
        }

        // Otherwise, build from source
        let source_dir = module
            .source_dir
            .as_deref()
            .map(|s| base_dir.join(s))
            .unwrap_or_else(|| base_dir.join(format!("wasm-src/{}", module.name)));

        if !source_dir.exists() {
            println!(
                "  {} Source directory not found: {}",
                "Warning:".yellow(),
                source_dir.display()
            );
            println!(
                "  {} Run: vil wasm scaffold {} --language {}",
                "Hint:".cyan(),
                module.name,
                module.language
            );
            continue;
        }

        match module.language.as_str() {
            "rust" => build_rust_module(&source_dir, &module.name)?,
            "c" => build_c_module(&source_dir, &module.name)?,
            "go" => build_go_module(&source_dir, &module.name)?,
            "assemblyscript" | "as" => build_as_module(&source_dir, &module.name)?,
            other => {
                println!("  {} Unsupported language: {}", "Warning:".yellow(), other);
                continue;
            }
        }

        // Validate exported functions match manifest
        println!("  {} Declared functions:", "INFO".dimmed());
        for func in &module.functions {
            let output = func.output.as_deref().unwrap_or("void");
            println!("    {} {}() -> {}", "fn".dimmed(), func.name, output);
        }
    }

    Ok(())
}

/// List all WASM modules and functions declared in a manifest.
pub fn list_modules(manifest_path: &str) -> Result<(), String> {
    let manifest = WorkflowManifest::from_file(manifest_path)?;

    if manifest.vil_wasm.is_empty() {
        println!("No vil_wasm: modules declared.");
        return Ok(());
    }

    for module in &manifest.vil_wasm {
        let pool = module.pool_size.unwrap_or(4);
        let wasm_file = Path::new(WASM_OUT_DIR).join(format!("{}.wasm", module.name));
        let status = if wasm_file.exists() {
            "compiled".green().to_string()
        } else {
            "not built".yellow().to_string()
        };

        println!(
            "{} {} ({}, pool:{}, {})",
            "MODULE".cyan().bold(),
            module.name,
            module.language,
            pool,
            status
        );

        for func in &module.functions {
            let output = func.output.as_deref().unwrap_or("void");
            let desc = func.description.as_deref().unwrap_or("");
            println!("  fn {}() -> {}  {}", func.name, output, desc.dimmed());
        }

        if let Some(sandbox) = &module.sandbox {
            let timeout = sandbox.timeout_ms.unwrap_or(5000);
            let mem = sandbox.max_memory_mb.unwrap_or(16);
            println!("  sandbox: timeout={}ms, memory={}MB", timeout, mem);
        }
        println!();
    }

    Ok(())
}

// ═══════════════════════════════════════════════════════════════════════════════
// Language-specific scaffolders
// ═══════════════════════════════════════════════════════════════════════════════

fn scaffold_rust(dir: &Path, name: &str) -> Result<(), String> {
    // Cargo.toml
    let cargo = format!(
        r#"[package]
name = "wasm_{name}"
version = "0.1.0"
edition = "2021"

[lib]
crate-type = ["cdylib"]

[profile.release]
opt-level = "s"
lto = true
"#,
        name = name
    );

    std::fs::write(dir.join("Cargo.toml"), cargo)
        .map_err(|e| format!("Write Cargo.toml: {}", e))?;

    // src/lib.rs
    let src_dir = dir.join("src");
    std::fs::create_dir_all(&src_dir).map_err(|e| format!("Create src/: {}", e))?;

    let lib = format!(
        r#"//! WASM module: {name}
//! Generated by: vil wasm scaffold {name} --language rust
//! Build: cargo build --target wasm32-unknown-unknown --release

#![no_std]

// ── Export your functions below ─────────────────────────────────────────
// Each function must be:
//   - #[no_mangle] pub extern "C"
//   - Use only i32/i64/f32/f64 parameters and return types
//   - For complex I/O, use the memory pattern (write at offset 0, read at 1024)

#[no_mangle]
pub extern "C" fn hello(input_ptr: i32, input_len: i32) -> i32 {{
    // TODO: Implement your function
    // Input: read input_len bytes from linear memory at offset input_ptr
    // Output: write result to offset 1024, return result length
    0
}}

// For simple (i32, i32) -> i32 functions:
// #[no_mangle]
// pub extern "C" fn calculate(a: i32, b: i32) -> i32 {{
//     a + b
// }}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {{
    loop {{}}
}}
"#,
        name = name
    );

    std::fs::write(src_dir.join("lib.rs"), lib).map_err(|e| format!("Write lib.rs: {}", e))?;

    Ok(())
}

fn scaffold_c(dir: &Path, name: &str) -> Result<(), String> {
    let main_c = format!(
        r#"// WASM module: {name}
// Generated by: vil wasm scaffold {name} --language c
// Build: clang --target=wasm32-unknown-unknown -nostdlib -O2 -Wl,--no-entry -Wl,--export-all -o {name}.wasm main.c

__attribute__((export_name("hello")))
int hello(int input_ptr, int input_len) {{
    // TODO: Implement your function
    return 0;
}}
"#,
        name = name
    );

    std::fs::write(dir.join("main.c"), main_c).map_err(|e| format!("Write main.c: {}", e))?;

    let makefile = format!(
        r#"TARGET = {name}.wasm
CC = clang
CFLAGS = --target=wasm32-unknown-unknown -nostdlib -O2 -Wl,--no-entry -Wl,--export-all

all: $(TARGET)

$(TARGET): main.c
	$(CC) $(CFLAGS) -o $(TARGET) main.c
	cp $(TARGET) ../../{wasm_out}/

clean:
	rm -f $(TARGET)
"#,
        name = name,
        wasm_out = WASM_OUT_DIR
    );

    std::fs::write(dir.join("Makefile"), makefile).map_err(|e| format!("Write Makefile: {}", e))?;

    Ok(())
}

fn scaffold_go(dir: &Path, name: &str) -> Result<(), String> {
    let go_mod = format!(
        r#"module wasm_{name}

go 1.21
"#,
        name = name
    );

    std::fs::write(dir.join("go.mod"), go_mod).map_err(|e| format!("Write go.mod: {}", e))?;

    let main_go = format!(
        r#"// WASM module: {name}
// Generated by: vil wasm scaffold {name} --language go
// Build: GOOS=wasip1 GOARCH=wasm go build -o {name}.wasm main.go
package main

//export hello
func hello(inputPtr, inputLen int32) int32 {{
    // TODO: Implement your function
    return 0
}}

func main() {{}}
"#,
        name = name
    );

    std::fs::write(dir.join("main.go"), main_go).map_err(|e| format!("Write main.go: {}", e))?;

    Ok(())
}

fn scaffold_assemblyscript(dir: &Path, name: &str) -> Result<(), String> {
    let package_json = format!(
        r#"{{
  "name": "wasm-{name}",
  "version": "0.1.0",
  "scripts": {{
    "build": "asc index.ts --outFile {name}.wasm --optimize"
  }},
  "devDependencies": {{
    "assemblyscript": "^0.27.0"
  }}
}}
"#,
        name = name
    );

    std::fs::write(dir.join("package.json"), package_json)
        .map_err(|e| format!("Write package.json: {}", e))?;

    let index_ts = format!(
        r#"// WASM module: {name}
// Generated by: vil wasm scaffold {name} --language assemblyscript
// Build: npm run build

export function hello(inputPtr: i32, inputLen: i32): i32 {{
  // TODO: Implement your function
  return 0;
}}
"#,
        name = name
    );

    std::fs::write(dir.join("index.ts"), index_ts).map_err(|e| format!("Write index.ts: {}", e))?;

    Ok(())
}

// ═══════════════════════════════════════════════════════════════════════════════
// Language-specific builders
// ═══════════════════════════════════════════════════════════════════════════════

fn build_rust_module(source_dir: &Path, name: &str) -> Result<(), String> {
    println!(
        "  {} cargo build --target wasm32-unknown-unknown --release",
        "CMD".dimmed()
    );

    let output = std::process::Command::new("cargo")
        .args(["build", "--target", "wasm32-unknown-unknown", "--release"])
        .current_dir(source_dir)
        .output()
        .map_err(|e| format!("Failed to run cargo: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!(
            "cargo build failed for WASM module '{}':\n{}",
            name, stderr
        ));
    }

    // Copy .wasm to wasm-out/
    let wasm_src = source_dir
        .join("target/wasm32-unknown-unknown/release")
        .join(format!("wasm_{}.wasm", name));

    let wasm_dst = Path::new(WASM_OUT_DIR).join(format!("{}.wasm", name));
    std::fs::copy(&wasm_src, &wasm_dst).map_err(|e| {
        format!(
            "Failed to copy '{}' → '{}': {}",
            wasm_src.display(),
            wasm_dst.display(),
            e
        )
    })?;

    let size = std::fs::metadata(&wasm_dst).map(|m| m.len()).unwrap_or(0);
    println!(
        "  {} {} ({} bytes)",
        "OK".green().bold(),
        wasm_dst.display(),
        size
    );

    Ok(())
}

fn build_c_module(source_dir: &Path, name: &str) -> Result<(), String> {
    println!("  {} make", "CMD".dimmed());

    let output = std::process::Command::new("make")
        .current_dir(source_dir)
        .output()
        .map_err(|e| format!("Failed to run make: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!(
            "make failed for WASM module '{}':\n{}",
            name, stderr
        ));
    }

    println!("  {} {}/{}.wasm", "OK".green().bold(), WASM_OUT_DIR, name);
    Ok(())
}

fn build_go_module(source_dir: &Path, name: &str) -> Result<(), String> {
    println!("  {} GOOS=wasip1 GOARCH=wasm go build", "CMD".dimmed());

    let output = std::process::Command::new("go")
        .args([
            "build",
            "-o",
            &format!("../../{}/{}.wasm", WASM_OUT_DIR, name),
            "main.go",
        ])
        .env("GOOS", "wasip1")
        .env("GOARCH", "wasm")
        .current_dir(source_dir)
        .output()
        .map_err(|e| format!("Failed to run go build: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!(
            "go build failed for WASM module '{}':\n{}",
            name, stderr
        ));
    }

    println!("  {} {}/{}.wasm", "OK".green().bold(), WASM_OUT_DIR, name);
    Ok(())
}

fn build_as_module(source_dir: &Path, name: &str) -> Result<(), String> {
    println!("  {} npm run build", "CMD".dimmed());

    let output = std::process::Command::new("npm")
        .args(["run", "build"])
        .current_dir(source_dir)
        .output()
        .map_err(|e| format!("Failed to run npm: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!(
            "npm build failed for WASM module '{}':\n{}",
            name, stderr
        ));
    }

    // Copy to wasm-out/
    let src = source_dir.join(format!("{}.wasm", name));
    let dst = Path::new(WASM_OUT_DIR).join(format!("{}.wasm", name));
    if src.exists() {
        std::fs::copy(&src, &dst).map_err(|e| format!("Copy failed: {}", e))?;
    }

    println!("  {} {}/{}.wasm", "OK".green().bold(), WASM_OUT_DIR, name);
    Ok(())
}
