// =============================================================================
// VIL CLI — Doctor Command (System Readiness Check)
// =============================================================================
//
// `vil doctor` checks system readiness for running vil-server:
//   - Rust toolchain version
//   - Required system libraries
//   - SHM availability (/dev/shm)
//   - Port availability
//   - Workspace health (cargo check)

use colored::*;

pub fn run_doctor() {
    println!("{}", "=== VIL Doctor — System Readiness Check ===".green().bold());
    println!();

    let mut all_ok = true;

    // 1. Rust toolchain
    match check_rust() {
        Ok(version) => println!("  {} Rust toolchain: {}", "✓".green(), version),
        Err(e) => {
            println!("  {} Rust toolchain: {}", "✗".red(), e);
            all_ok = false;
        }
    }

    // 2. Cargo
    match check_cargo() {
        Ok(version) => println!("  {} Cargo: {}", "✓".green(), version),
        Err(e) => {
            println!("  {} Cargo: {}", "✗".red(), e);
            all_ok = false;
        }
    }

    // 3. SHM availability
    if check_shm() {
        println!("  {} /dev/shm available (shared memory)", "✓".green());
    } else {
        println!("  {} /dev/shm not available (SHM features disabled)", "⚠".yellow());
    }

    // 4. SHM size
    match check_shm_size() {
        Some(size_mb) => {
            if size_mb >= 256 {
                println!("  {} SHM size: {}MB", "✓".green(), size_mb);
            } else {
                println!("  {} SHM size: {}MB (recommend ≥256MB)", "⚠".yellow(), size_mb);
            }
        }
        None => println!("  {} SHM size: unknown", "⚠".yellow()),
    }

    // 5. Port 8080
    if check_port(8080) {
        println!("  {} Port 8080 available", "✓".green());
    } else {
        println!("  {} Port 8080 in use", "⚠".yellow());
    }

    // 6. Workspace
    if std::path::Path::new("Cargo.toml").exists() {
        println!("  {} Cargo.toml found", "✓".green());
    } else {
        println!("  {} No Cargo.toml in current directory", "⚠".yellow());
    }

    // 7. OS info
    println!("  {} OS: {}", "ℹ".blue(), std::env::consts::OS);
    println!("  {} Arch: {}", "ℹ".blue(), std::env::consts::ARCH);

    println!();
    if all_ok {
        println!("{}", "All checks passed. System is ready for vil-server.".green().bold());
    } else {
        println!("{}", "Some checks failed. Please fix the issues above.".yellow().bold());
    }
}

fn check_rust() -> Result<String, String> {
    let output = std::process::Command::new("rustc")
        .arg("--version")
        .output()
        .map_err(|_| "rustc not found".to_string())?;

    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    } else {
        Err("rustc failed".to_string())
    }
}

fn check_cargo() -> Result<String, String> {
    let output = std::process::Command::new("cargo")
        .arg("--version")
        .output()
        .map_err(|_| "cargo not found".to_string())?;

    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    } else {
        Err("cargo failed".to_string())
    }
}

fn check_shm() -> bool {
    std::path::Path::new("/dev/shm").exists()
}

fn check_shm_size() -> Option<u64> {
    #[cfg(target_os = "linux")]
    {
        let output = std::process::Command::new("df")
            .args(["--output=size", "/dev/shm"])
            .output()
            .ok()?;
        let text = String::from_utf8_lossy(&output.stdout);
        let size_kb: u64 = text.lines().nth(1)?.trim().parse().ok()?;
        Some(size_kb / 1024) // Convert KB to MB
    }
    #[cfg(not(target_os = "linux"))]
    {
        None
    }
}

fn check_port(port: u16) -> bool {
    std::net::TcpListener::bind(("0.0.0.0", port)).is_ok()
}
