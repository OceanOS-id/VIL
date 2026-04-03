//! vil dev — development mode with auto-rebuild
//!
//! Watches src/ and migrations/ for file changes and automatically rebuilds + restarts.
//! Enhanced: clear screen, colored output, port management, migration auto-detect.

use colored::Colorize;
use std::collections::HashMap;
use std::path::Path;
use std::process::{Child, Command};
use std::time::{Duration, Instant, SystemTime};

pub struct DevConfig {
    pub port: u16,
    pub package: Option<String>,
    pub interval: u64,
}

pub fn run_dev(config: DevConfig) -> Result<(), String> {
    clear_screen();
    println!();
    println!("  {}", "╔══════════════════════════════════════════════════╗".cyan());
    println!("  {}  {} — Development Mode                     {}", "║".cyan(), "vil dev".green().bold(), "║".cyan());
    println!("  {}", "╚══════════════════════════════════════════════════╝".cyan());
    println!();

    let package = config
        .package
        .unwrap_or_else(|| read_package_name().unwrap_or_else(|| "app".to_string()));

    let interval = if config.interval > 0 { config.interval } else { 500 };

    println!("  {}   {}", "Package:".dimmed(), package.cyan());
    println!("  {}      {}", "Port:".dimmed(), config.port.to_string().cyan());
    println!("  {}  {}ms", "Interval:".dimmed(), interval);
    println!("  {}  {}", "Watching:".dimmed(), "src/, migrations/".cyan());
    println!();

    let mut child: Option<Child> = None;
    let mut last_src = collect_mtimes("src/");
    let mut last_mig = collect_mtimes("migrations/");

    // Initial build and run
    print_status("build", &format!("Compiling {}...", package));
    let start = Instant::now();
    match build_and_run(&package, &mut child) {
        Ok(_) => {
            let elapsed = start.elapsed();
            print_status("ready", &format!(
                "http://localhost:{} ({:.1}s)",
                config.port,
                elapsed.as_secs_f64()
            ));
            print_status("info", &format!(
                "Dashboard: http://localhost:{}/_vil/dashboard/",
                config.port
            ));
        }
        Err(e) => print_error(&e),
    }

    println!();
    println!("  {} Watching for changes... (Ctrl+C to stop)", "👀".dimmed());

    // Watch loop
    loop {
        std::thread::sleep(Duration::from_millis(interval));

        let current_src = collect_mtimes("src/");
        let current_mig = collect_mtimes("migrations/");
        let src_changed = has_changes(&last_src, &current_src);
        let mig_changed = has_changes(&last_mig, &current_mig);

        if src_changed || mig_changed {
            println!();

            if mig_changed {
                print_status("migrate", "Migration files changed");
            }
            if src_changed {
                print_status("change", "Source files modified");
            }

            // Kill old process
            if let Some(ref mut c) = child {
                let _ = c.kill();
                let _ = c.wait();
            }
            child = None;

            // Rebuild
            print_status("build", &format!("Recompiling {}...", package));
            let start = Instant::now();
            match build_and_run(&package, &mut child) {
                Ok(_) => {
                    let elapsed = start.elapsed();
                    print_status("ready", &format!(
                        "http://localhost:{} ({:.1}s)",
                        config.port,
                        elapsed.as_secs_f64()
                    ));
                }
                Err(e) => print_error(&e),
            }

            last_src = current_src;
            last_mig = current_mig;
        }
    }
}

fn print_status(tag: &str, msg: &str) {
    let colored_tag = match tag {
        "ready" => format!("  {} {}", "✅".green(), msg.green()),
        "build" => format!("  {} {}", "🔨".yellow(), msg.yellow()),
        "change" => format!("  {} {}", "📝".cyan(), msg.cyan()),
        "migrate" => format!("  {} {}", "🗄️ ".blue(), msg.blue()),
        "error" => format!("  {} {}", "❌".red(), msg.red()),
        "info" => format!("  {} {}", "ℹ️ ".dimmed(), msg.dimmed()),
        _ => format!("  [{}] {}", tag, msg),
    };
    println!("{}", colored_tag);
}

fn print_error(msg: &str) {
    println!("  {} {}", "❌ Build failed:".red().bold(), msg.red());
    println!("  {} Fix errors and save to retry", "→".dimmed());
}

fn clear_screen() {
    print!("\x1B[2J\x1B[1;1H");
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
    let build = Command::new("cargo")
        .args(["build", "-p", package])
        .status()
        .map_err(|e| format!("cargo build: {}", e))?;

    if !build.success() {
        return Err("Compilation failed — see errors above".into());
    }

    // Find binary (try both formats)
    let binary = format!("target/debug/{}", package);
    let binary_alt = format!("target/debug/{}", package.replace('-', "_"));
    let bin_path = if Path::new(&binary).exists() {
        binary
    } else if Path::new(&binary_alt).exists() {
        binary_alt
    } else {
        return Err(format!("Binary not found: {} or {}", binary, binary_alt));
    };

    let c = Command::new(&bin_path)
        .spawn()
        .map_err(|e| format!("start {}: {}", bin_path, e))?;

    *child = Some(c);
    Ok(())
}

fn collect_mtimes(dir: &str) -> HashMap<String, SystemTime> {
    let mut map = HashMap::new();
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() {
                let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
                if matches!(ext, "rs" | "sql" | "toml") {
                    if let Ok(meta) = path.metadata() {
                        if let Ok(mtime) = meta.modified() {
                            map.insert(path.to_string_lossy().to_string(), mtime);
                        }
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
