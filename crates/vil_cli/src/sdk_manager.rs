//! SDK manager — download, install, and manage VIL SDK distributions.
//!
//! `vil sdk install [--version latest]` — download SDK to ~/.vil/sdk/
//! `vil sdk info` — show installed version + path
//! `vil sdk path` — print SDK path (for scripts)

use colored::*;
use std::path::{Path, PathBuf};

const SDK_BASE_URL: &str = "https://github.com/nicholasgasior/vil-community/releases/download";
const SDK_DIR_NAME: &str = ".vil/sdk";

/// Get the SDK base directory (~/.vil/sdk/).
pub fn sdk_base_dir() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(SDK_DIR_NAME)
}

/// Get the current SDK path (~/.vil/sdk/current/).
pub fn sdk_current_path() -> PathBuf {
    sdk_base_dir().join("current")
}

/// Check if SDK is installed.
pub fn is_sdk_installed() -> bool {
    let current = sdk_current_path();
    current.exists() && current.join("libs").exists()
}

/// Get the SDK libs directory.
pub fn sdk_libs_path() -> Option<PathBuf> {
    let p = sdk_current_path().join("libs");
    if p.exists() { Some(p) } else { None }
}

/// Get the SDK internal crates directory.
pub fn sdk_internal_path() -> Option<PathBuf> {
    let p = sdk_current_path().join("internal");
    if p.exists() { Some(p) } else { None }
}

/// Detect current platform for SDK download.
fn detect_platform() -> &'static str {
    #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
    { "x86_64-unknown-linux-gnu" }
    #[cfg(all(target_os = "macos", target_arch = "x86_64"))]
    { "x86_64-apple-darwin" }
    #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
    { "aarch64-apple-darwin" }
    #[cfg(all(target_os = "windows", target_arch = "x86_64"))]
    { "x86_64-pc-windows-msvc" }
    #[cfg(not(any(
        all(target_os = "linux", target_arch = "x86_64"),
        all(target_os = "macos", target_arch = "x86_64"),
        all(target_os = "macos", target_arch = "aarch64"),
        all(target_os = "windows", target_arch = "x86_64"),
    )))]
    { "unknown" }
}

/// Install SDK from GitHub Releases or local sdk-dist/.
pub fn install_sdk(version: &str) -> Result<(), String> {
    let base_dir = sdk_base_dir();
    let version_dir = base_dir.join(version);
    let current_link = base_dir.join("current");
    let platform = detect_platform();

    println!("{} Installing VIL SDK v{} ({})", ">>>".cyan().bold(), version, platform);

    // Create base directory
    std::fs::create_dir_all(&base_dir)
        .map_err(|e| format!("Failed to create SDK directory: {}", e))?;

    // Check for local sdk-dist/ first (development mode)
    let local_sdk = PathBuf::from("sdk-dist");
    if local_sdk.exists() && local_sdk.join("libs").exists() {
        println!("  {} Found local sdk-dist/, installing from local source", "OK".green());
        copy_dir_recursive(&local_sdk, &version_dir)?;
    } else {
        // Download from GitHub Releases
        let archive_name = format!("vil-sdk-{}-{}.tar.gz", version, platform);
        let url = format!("{}/v{}/{}", SDK_BASE_URL, version, archive_name);
        let tmp_archive = std::env::temp_dir().join(&archive_name);

        println!("  {} Downloading from {}", "GET".dimmed(), url);

        let output = std::process::Command::new("curl")
            .args(["-fSL", "-o", tmp_archive.to_str().unwrap(), &url])
            .output()
            .map_err(|e| format!("curl failed: {}", e))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(format!(
                "Download failed. URL: {}\n  {}\n  \
                 If building from source, run: ./scripts/pack_sdk.sh",
                url, stderr.trim()
            ));
        }

        // Extract archive
        println!("  {} Extracting to {}", "TAR".dimmed(), version_dir.display());
        std::fs::create_dir_all(&version_dir)
            .map_err(|e| format!("Failed to create version dir: {}", e))?;

        let output = std::process::Command::new("tar")
            .args(["xzf", tmp_archive.to_str().unwrap(), "-C", version_dir.to_str().unwrap()])
            .output()
            .map_err(|e| format!("tar failed: {}", e))?;

        if !output.status.success() {
            return Err("tar extraction failed".into());
        }

        // Cleanup
        let _ = std::fs::remove_file(&tmp_archive);
    }

    // Create/update symlink: current → version
    let _ = std::fs::remove_file(&current_link);
    #[cfg(unix)]
    std::os::unix::fs::symlink(&version_dir, &current_link)
        .map_err(|e| format!("Failed to create symlink: {}", e))?;
    #[cfg(windows)]
    std::os::windows::fs::symlink_dir(&version_dir, &current_link)
        .map_err(|e| format!("Failed to create symlink: {}", e))?;

    // Verify
    let libs_ok = version_dir.join("libs").exists();
    let internal_ok = version_dir.join("internal").exists();

    println!();
    println!("  {} SDK v{} installed at {}", "OK".green().bold(), version, version_dir.display());
    println!("  {} libs/      {}", if libs_ok { "OK".green() } else { "MISSING".red() }, version_dir.join("libs").display());
    println!("  {} internal/  {}", if internal_ok { "OK".green() } else { "MISSING".red() }, version_dir.join("internal").display());
    println!("  {} current -> {}", "LINK".dimmed(), version_dir.display());

    if libs_ok && internal_ok {
        println!("\n{} SDK ready. `vil compile` will use pre-compiled engine.", "DONE".green().bold());
    } else {
        println!("\n{} SDK partially installed — some directories missing.", "WARN".yellow().bold());
    }

    Ok(())
}

/// Show SDK information.
pub fn show_info() -> Result<(), String> {
    let base = sdk_base_dir();
    let current = sdk_current_path();

    println!("{} VIL SDK", "INFO".cyan().bold());
    println!("  Base directory: {}", base.display());
    println!("  Platform:       {}", detect_platform());

    if !current.exists() {
        println!("  Status:         {} (run: vil sdk install)", "NOT INSTALLED".yellow());
        return Ok(());
    }

    // Read version from symlink target
    let target = std::fs::read_link(&current)
        .map(|p| p.file_name().unwrap_or_default().to_string_lossy().to_string())
        .unwrap_or_else(|_| "unknown".into());

    println!("  Version:        {}", target.green());
    println!("  Path:           {}", current.display());

    // Check contents
    let libs = current.join("libs");
    if libs.exists() {
        let engine_a = libs.join("libvil_engine.a");
        let runtime_a = libs.join("libvil_runtime.a");
        let engine_size = std::fs::metadata(&engine_a).map(|m| m.len()).unwrap_or(0);
        let runtime_size = std::fs::metadata(&runtime_a).map(|m| m.len()).unwrap_or(0);
        println!("  libvil_engine.a:  {} ({:.1} MB)", if engine_a.exists() { "OK".green() } else { "MISSING".red() }, engine_size as f64 / 1_048_576.0);
        println!("  libvil_runtime.a: {} ({:.1} MB)", if runtime_a.exists() { "OK".green() } else { "MISSING".red() }, runtime_size as f64 / 1_048_576.0);
    } else {
        println!("  libs/: {}", "MISSING".red());
    }

    let internal = current.join("internal");
    if internal.exists() {
        let crate_count = std::fs::read_dir(&internal)
            .map(|d| d.filter_map(|e| e.ok()).filter(|e| e.path().is_dir()).count())
            .unwrap_or(0);
        println!("  internal/:      {} crates", crate_count);
    }

    Ok(())
}

/// Print SDK path (for scripts).
pub fn show_path() -> Result<(), String> {
    let current = sdk_current_path();
    if current.exists() {
        println!("{}", current.display());
    } else {
        return Err("SDK not installed. Run: vil sdk install".into());
    }
    Ok(())
}

/// List installed SDK versions.
pub fn list_versions() -> Result<(), String> {
    let base = sdk_base_dir();
    if !base.exists() {
        println!("No SDK versions installed.");
        return Ok(());
    }

    let current_target = std::fs::read_link(base.join("current"))
        .ok()
        .and_then(|p| p.file_name().map(|f| f.to_string_lossy().to_string()));

    let entries = std::fs::read_dir(&base)
        .map_err(|e| format!("Failed to read SDK dir: {}", e))?;

    println!("{} Installed SDK versions:", "INFO".cyan().bold());
    for entry in entries {
        let entry = entry.map_err(|e| e.to_string())?;
        let name = entry.file_name().to_string_lossy().to_string();
        if name == "current" { continue; }
        if !entry.path().is_dir() { continue; }

        let is_current = current_target.as_deref() == Some(&name);
        let marker = if is_current { " ← current".green().to_string() } else { String::new() };
        println!("  {}{}", name, marker);
    }

    Ok(())
}

/// Copy directory recursively.
fn copy_dir_recursive(src: &Path, dst: &Path) -> Result<(), String> {
    std::fs::create_dir_all(dst)
        .map_err(|e| format!("mkdir {}: {}", dst.display(), e))?;

    for entry in std::fs::read_dir(src).map_err(|e| format!("readdir {}: {}", src.display(), e))? {
        let entry = entry.map_err(|e| e.to_string())?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());

        if src_path.is_dir() {
            copy_dir_recursive(&src_path, &dst_path)?;
        } else {
            std::fs::copy(&src_path, &dst_path)
                .map_err(|e| format!("copy {} → {}: {}", src_path.display(), dst_path.display(), e))?;
        }
    }
    Ok(())
}
