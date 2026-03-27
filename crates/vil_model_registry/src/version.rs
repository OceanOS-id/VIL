/// Utility for version management.

/// Get the next version number given existing entries.
pub fn next_version(current_max: u32) -> u32 {
    current_max + 1
}

/// Format a version for display.
pub fn format_version(name: &str, version: u32) -> String {
    format!("{}@v{}", name, version)
}
