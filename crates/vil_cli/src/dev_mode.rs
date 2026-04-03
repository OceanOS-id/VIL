//! vil dev — development mode with auto-rebuild
//!
//! Watches src/, migrations/, and .env for file changes and automatically rebuilds + restarts.
//!
//! Features:
//!   - Event-based file watching (notify/inotify) — no polling
//!   - Forwards env vars + loads .env file
//!   - Graceful shutdown (SIGTERM → timeout → SIGKILL)
//!   - Passes PORT to child process
//!   - Handles cargo workspace (reads [[bin]] name)
//!   - Sets VIL_DEV_MODE=1 to suppress tracing double-init

use colored::Colorize;
use notify::{RecursiveMode, Watcher};
use std::collections::HashMap;
use std::path::Path;
use std::process::{Child, Command};
use std::sync::mpsc;
use std::time::{Duration, Instant};

pub struct DevConfig {
    pub port: u16,
    pub package: Option<String>,
    pub interval: u64,
}

pub fn run_dev(config: DevConfig) -> Result<(), String> {
    clear_screen();
    println!();
    println!("  {}", "╔══════════════════════════════════════════════════╗".cyan());
    println!(
        "  {}  {} — Development Mode                     {}",
        "║".cyan(),
        "vil dev".green().bold(),
        "║".cyan()
    );
    println!("  {}", "╚══════════════════════════════════════════════════╝".cyan());
    println!();

    // Load .env file if present (fix #5)
    let dotenv = load_dotenv();

    let package = config
        .package
        .unwrap_or_else(|| read_package_name().unwrap_or_else(|| "app".to_string()));

    // Resolve binary name from Cargo.toml [[bin]] section (fix #6)
    let binary_name = read_binary_name().unwrap_or_else(|| package.clone());

    let debounce_ms = if config.interval > 0 {
        config.interval
    } else {
        300
    };

    println!("  {}   {}", "Package:".dimmed(), package.cyan());
    println!("  {}    {}", "Binary:".dimmed(), binary_name.cyan());
    println!("  {}      {}", "Port:".dimmed(), config.port.to_string().cyan());
    println!(
        "  {}  {}ms",
        "Debounce:".dimmed(),
        debounce_ms
    );
    println!(
        "  {}  {}",
        "Watching:".dimmed(),
        "src/, migrations/, .env".cyan()
    );
    if !dotenv.is_empty() {
        println!(
            "  {}    {} vars from .env",
            "Dotenv:".dimmed(),
            dotenv.len().to_string().cyan()
        );
    }
    println!();

    // Set up notify watcher (fix #1 — event-based, not polling)
    let (tx, rx) = mpsc::channel();

    let mut watcher = notify::recommended_watcher(move |res: Result<notify::Event, notify::Error>| {
        if let Ok(event) = res {
            // Only care about modify/create/remove
            use notify::EventKind::*;
            match event.kind {
                Modify(_) | Create(_) | Remove(_) => {
                    // Filter by extension
                    let dominated = event.paths.iter().any(|p| {
                        let ext = p.extension().and_then(|e| e.to_str()).unwrap_or("");
                        matches!(ext, "rs" | "sql" | "toml" | "env")
                    });
                    if dominated {
                        let _ = tx.send(event);
                    }
                }
                _ => {}
            }
        }
    })
    .map_err(|e| format!("Failed to create file watcher: {e}"))?;

    // Watch directories
    if Path::new("src").exists() {
        watcher
            .watch(Path::new("src"), RecursiveMode::Recursive)
            .map_err(|e| format!("Watch src/: {e}"))?;
    }
    if Path::new("migrations").exists() {
        watcher
            .watch(Path::new("migrations"), RecursiveMode::Recursive)
            .map_err(|e| format!("Watch migrations/: {e}"))?;
    }
    if Path::new(".env").exists() {
        watcher
            .watch(Path::new(".env"), RecursiveMode::NonRecursive)
            .map_err(|e| format!("Watch .env: {e}"))?;
    }
    // Watch Cargo.toml for dependency changes
    if Path::new("Cargo.toml").exists() {
        watcher
            .watch(Path::new("Cargo.toml"), RecursiveMode::NonRecursive)
            .map_err(|e| format!("Watch Cargo.toml: {e}"))?;
    }

    let mut child: Option<Child> = None;

    // Initial build and run
    print_status("build", &format!("Compiling {}...", package));
    let start = Instant::now();
    match build_and_run(&package, &binary_name, config.port, &dotenv, &mut child) {
        Ok(_) => {
            let elapsed = start.elapsed();
            print_status(
                "ready",
                &format!(
                    "http://localhost:{} ({:.1}s)",
                    config.port,
                    elapsed.as_secs_f64()
                ),
            );
            print_status(
                "info",
                &format!(
                    "Dashboard: http://localhost:{}/_vil/dashboard/",
                    config.port
                ),
            );
        }
        Err(e) => print_error(&e),
    }

    println!();
    println!(
        "  {} Watching for changes... (Ctrl+C to stop)",
        "👀".dimmed()
    );

    // Event loop with debounce
    loop {
        // Block until first event
        match rx.recv() {
            Ok(event) => {
                // Debounce: drain remaining events within debounce window
                let deadline = Instant::now() + Duration::from_millis(debounce_ms);
                let mut has_migration = is_migration_event(&event);
                let mut has_env = is_env_event(&event);

                while Instant::now() < deadline {
                    match rx.recv_timeout(deadline - Instant::now()) {
                        Ok(ev) => {
                            has_migration = has_migration || is_migration_event(&ev);
                            has_env = has_env || is_env_event(&ev);
                        }
                        Err(_) => break,
                    }
                }

                println!();

                if has_env {
                    print_status("env", "Environment changed — reloading .env");
                }
                if has_migration {
                    print_status("migrate", "Migration files changed");
                } else {
                    print_status("change", "Source files modified");
                }

                // Reload .env if changed (fix #5)
                let current_dotenv = if has_env { load_dotenv() } else { dotenv.clone() };

                // Graceful shutdown (fix #3)
                graceful_stop(&mut child);

                // Rebuild
                print_status("build", &format!("Recompiling {}...", package));
                let start = Instant::now();
                match build_and_run(
                    &package,
                    &binary_name,
                    config.port,
                    &current_dotenv,
                    &mut child,
                ) {
                    Ok(_) => {
                        let elapsed = start.elapsed();
                        print_status(
                            "ready",
                            &format!(
                                "http://localhost:{} ({:.1}s)",
                                config.port,
                                elapsed.as_secs_f64()
                            ),
                        );
                    }
                    Err(e) => print_error(&e),
                }
            }
            Err(_) => break,
        }
    }

    // Cleanup on exit
    graceful_stop(&mut child);
    Ok(())
}

// ── Helpers ──

fn print_status(tag: &str, msg: &str) {
    let colored_tag = match tag {
        "ready" => format!("  {} {}", "✅".green(), msg.green()),
        "build" => format!("  {} {}", "🔨".yellow(), msg.yellow()),
        "change" => format!("  {} {}", "📝".cyan(), msg.cyan()),
        "migrate" => format!("  {} {}", "🗄️ ".blue(), msg.blue()),
        "env" => format!("  {} {}", "🔄".blue(), msg.blue()),
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

fn is_migration_event(event: &notify::Event) -> bool {
    event
        .paths
        .iter()
        .any(|p| p.to_string_lossy().contains("migrations"))
}

fn is_env_event(event: &notify::Event) -> bool {
    event.paths.iter().any(|p| {
        p.file_name()
            .and_then(|n| n.to_str())
            .map(|n| n == ".env")
            .unwrap_or(false)
    })
}

/// Load .env file into a HashMap (fix #5).
/// Does NOT pollute current process env — we pass explicitly to child.
fn load_dotenv() -> HashMap<String, String> {
    let mut map = HashMap::new();
    if let Ok(iter) = dotenvy::from_filename_iter(".env") {
        for item in iter.flatten() {
            map.insert(item.0, item.1);
        }
    }
    map
}

/// Read `name = "..."` from [package] in Cargo.toml.
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

/// Read binary name from [[bin]] section in Cargo.toml (fix #6).
/// Falls back to package name if no [[bin]] section found.
fn read_binary_name() -> Option<String> {
    let content = std::fs::read_to_string("Cargo.toml").ok()?;
    let mut in_bin_section = false;
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed == "[[bin]]" {
            in_bin_section = true;
            continue;
        }
        if in_bin_section && trimmed.starts_with("name") {
            if let Some(val) = trimmed.split('=').nth(1) {
                return Some(val.trim().trim_matches('"').to_string());
            }
        }
        // Exit bin section on new section header
        if in_bin_section && trimmed.starts_with('[') && trimmed != "[[bin]]" {
            in_bin_section = false;
        }
    }
    None
}

fn build_and_run(
    package: &str,
    binary_name: &str,
    port: u16,
    dotenv: &HashMap<String, String>,
    child: &mut Option<Child>,
) -> Result<(), String> {
    let build = Command::new("cargo")
        .args(["build", "-p", package])
        .status()
        .map_err(|e| format!("cargo build: {e}"))?;

    if !build.success() {
        return Err("Compilation failed — see errors above".into());
    }

    // Find binary — try dash and underscore variants (fix #6)
    let candidates = [
        format!("target/debug/{}", binary_name),
        format!("target/debug/{}", binary_name.replace('-', "_")),
        format!("target/debug/{}", package),
        format!("target/debug/{}", package.replace('-', "_")),
    ];

    let bin_path = candidates
        .iter()
        .find(|p| Path::new(p).exists())
        .ok_or_else(|| {
            format!(
                "Binary not found. Tried: {}",
                candidates.join(", ")
            )
        })?;

    // Build env vars for child process (fix #2 + #4 + #7)
    let mut cmd = Command::new(bin_path);

    // Forward all current process env vars (fix #2)
    cmd.envs(std::env::vars());

    // Layer .env vars on top (fix #5) — .env overrides current env
    cmd.envs(dotenv.iter());

    // Always set PORT (fix #4)
    cmd.env("PORT", port.to_string());

    // Signal to VIL that we're in dev mode — suppress tracing double-init (fix #7)
    cmd.env("VIL_DEV_MODE", "1");

    let c = cmd
        .spawn()
        .map_err(|e| format!("start {}: {e}", bin_path))?;

    *child = Some(c);
    Ok(())
}

/// Graceful shutdown: SIGTERM → wait 5s → SIGKILL (fix #3).
fn graceful_stop(child: &mut Option<Child>) {
    if let Some(ref mut c) = child {
        let pid = c.id();

        // Send SIGTERM first (Unix only)
        #[cfg(unix)]
        {
            unsafe {
                libc::kill(pid as i32, libc::SIGTERM);
            }
        }

        // Wait up to 5 seconds for graceful exit
        let deadline = Instant::now() + Duration::from_secs(5);
        loop {
            match c.try_wait() {
                Ok(Some(_)) => break, // exited
                Ok(None) => {
                    if Instant::now() >= deadline {
                        // Timeout — force kill
                        let _ = c.kill();
                        let _ = c.wait();
                        break;
                    }
                    std::thread::sleep(Duration::from_millis(100));
                }
                Err(_) => {
                    let _ = c.kill();
                    break;
                }
            }
        }
    }
    *child = None;
}
