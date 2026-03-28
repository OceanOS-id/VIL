fn main() {
    // Capture rustc version at build time
    let output = std::process::Command::new("rustc")
        .arg("--version")
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_else(|_| "unknown".into());
    // e.g. "rustc 1.93.1 (4eb161df4 2025-01-20)"
    let ver = output.split_whitespace().nth(1).unwrap_or("unknown");
    println!("cargo:rustc-env=VIL_RUST_VERSION={}", ver);
}
