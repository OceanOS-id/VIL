// =============================================================================
// VIL CLI — Server Dev Mode (auto-restart on file changes)
// =============================================================================
//
// `vil server dev` watches for file changes and auto-restarts the server.
// Similar to `cargo-watch` but integrated with vil-server lifecycle.
//
// Implementation: spawns `cargo run` and restarts on src/ changes.
// Uses polling-based file watcher for portability.

use std::process::{Child, Command};
use std::time::{Duration, SystemTime};

pub fn run_dev_mode(port: u16) -> Result<(), String> {
    println!("  vil-server dev mode");
    println!("  Watching for changes in src/");
    println!("  Port: {}", port);
    println!("  Press Ctrl+C to stop");
    println!();

    let mut child = start_server(port)?;
    let mut last_modified = get_src_mtime();

    loop {
        std::thread::sleep(Duration::from_secs(2));

        let current_mtime = get_src_mtime();

        if current_mtime > last_modified {
            println!("\n  File change detected — restarting...\n");

            // Kill old process
            let _ = child.kill();
            let _ = child.wait();

            // Restart
            match start_server(port) {
                Ok(new_child) => {
                    child = new_child;
                    last_modified = current_mtime;
                }
                Err(e) => {
                    eprintln!("  Failed to restart: {}", e);
                    eprintln!("  Waiting for next change...");
                    last_modified = current_mtime;
                }
            }
        }

        // Check if child has exited unexpectedly
        match child.try_wait() {
            Ok(Some(status)) => {
                if !status.success() {
                    eprintln!("  Server exited with status: {}", status);
                    eprintln!("  Waiting for file changes to restart...");
                    // Wait for a change before restarting
                    loop {
                        std::thread::sleep(Duration::from_secs(2));
                        let new_mtime = get_src_mtime();
                        if new_mtime > last_modified {
                            last_modified = new_mtime;
                            break;
                        }
                    }
                    match start_server(port) {
                        Ok(new_child) => child = new_child,
                        Err(e) => eprintln!("  Failed to restart: {}", e),
                    }
                }
            }
            Ok(None) => {} // Still running
            Err(e) => {
                eprintln!("  Error checking process: {}", e);
            }
        }
    }
}

fn start_server(port: u16) -> Result<Child, String> {
    Command::new("cargo")
        .args(["run", "--", "--port", &port.to_string()])
        .env("RUST_LOG", "info")
        .spawn()
        .map_err(|e| format!("Failed to start server: {}", e))
}

fn get_src_mtime() -> SystemTime {
    let mut latest = SystemTime::UNIX_EPOCH;

    if let Ok(entries) = std::fs::read_dir("src") {
        for entry in entries.flatten() {
            if let Ok(meta) = entry.metadata() {
                if let Ok(mtime) = meta.modified() {
                    if mtime > latest {
                        latest = mtime;
                    }
                }
            }
        }
    }

    // Also check Cargo.toml
    if let Ok(meta) = std::fs::metadata("Cargo.toml") {
        if let Ok(mtime) = meta.modified() {
            if mtime > latest {
                latest = mtime;
            }
        }
    }

    latest
}
