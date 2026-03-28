//! vil dev — development mode with auto-rebuild
//!
//! Watches src/ for file changes and automatically rebuilds + restarts.
//! This is for DEVELOPMENT only — not production hot-reload.

use std::collections::HashMap;
use std::path::Path;
use std::process::{Child, Command};
use std::time::{Duration, SystemTime};

pub struct DevConfig {
    pub port: u16,
    pub package: Option<String>,
    pub interval: u64,
}

pub fn run_dev(config: DevConfig) -> Result<(), String> {
    println!();
    println!("  ╔══════════════════════════════════════════════════╗");
    println!("  ║  vil dev — Development Mode                   ║");
    println!("  ╚══════════════════════════════════════════════════╝");
    println!();

    // Read package name from Cargo.toml if not specified
    let package = config
        .package
        .unwrap_or_else(|| read_package_name().unwrap_or_else(|| "app".to_string()));

    println!("  Package:   {}", package);
    println!("  Port:      {}", config.port);
    println!("  Interval:  {}ms", config.interval);
    println!("  Watching:  src/");
    println!();

    let mut child: Option<Child> = None;
    let mut last_modified = collect_mtimes("src/");

    // Initial build and run
    println!("  [build] Compiling {}...", package);
    match build_and_run(&package, &mut child) {
        Ok(_) => println!("  [ready] http://localhost:{}", config.port),
        Err(e) => println!("  [error] Build failed: {}", e),
    }

    // Watch loop
    loop {
        std::thread::sleep(Duration::from_millis(config.interval));

        let current = collect_mtimes("src/");
        if has_changes(&last_modified, &current) {
            println!();
            println!("  [change] Source files modified");

            // Kill old process
            if let Some(ref mut c) = child {
                let _ = c.kill();
                let _ = c.wait();
            }
            child = None;

            // Rebuild
            println!("  [build] Recompiling {}...", package);
            match build_and_run(&package, &mut child) {
                Ok(_) => println!("  [ready] http://localhost:{}", config.port),
                Err(e) => println!("  [error] Build failed: {}", e),
            }

            last_modified = current;
        }
    }
}

fn read_package_name() -> Option<String> {
    let content = std::fs::read_to_string("Cargo.toml").ok()?;
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("name") {
            if let Some(val) = trimmed.split('=').nth(1) {
                return Some(val.trim().trim_matches('"').to_string());
            }
        }
    }
    None
}

fn build_and_run(package: &str, child: &mut Option<Child>) -> Result<(), String> {
    // Build
    let build = Command::new("cargo")
        .args(["build", "-p", package])
        .status()
        .map_err(|e| format!("Failed to run cargo build: {}", e))?;

    if !build.success() {
        return Err("Compilation failed".into());
    }

    // Find binary
    let binary = format!("target/debug/{}", package.replace('-', "_"));
    if !Path::new(&binary).exists() {
        return Err(format!("Binary not found: {}", binary));
    }

    // Run
    let c = Command::new(&binary)
        .spawn()
        .map_err(|e| format!("Failed to start {}: {}", binary, e))?;

    *child = Some(c);
    Ok(())
}

fn collect_mtimes(dir: &str) -> HashMap<String, SystemTime> {
    let mut map = HashMap::new();
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() && path.extension().map(|e| e == "rs").unwrap_or(false) {
                if let Ok(meta) = path.metadata() {
                    if let Ok(mtime) = meta.modified() {
                        map.insert(path.to_string_lossy().to_string(), mtime);
                    }
                }
            } else if path.is_dir() {
                map.extend(collect_mtimes(path.to_str().unwrap_or("")));
            }
        }
    }
    map
}

fn has_changes(old: &HashMap<String, SystemTime>, new: &HashMap<String, SystemTime>) -> bool {
    if old.len() != new.len() {
        return true;
    }
    for (path, mtime) in new {
        match old.get(path) {
            Some(old_mtime) if old_mtime == mtime => continue,
            _ => return true,
        }
    }
    false
}
