// =============================================================================
// vil_edge_deploy::targets — Supported cross-compile targets
// =============================================================================

use serde::{Deserialize, Serialize};

/// Supported cross-compilation targets.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum EdgeTarget {
    /// aarch64-unknown-linux-gnu (ARM64, Raspberry Pi 4, Jetson, etc.)
    Aarch64Linux,
    /// armv7-unknown-linux-gnueabihf (ARMv7 hard-float, Raspberry Pi 2/3)
    Armv7Linux,
    /// riscv64gc-unknown-linux-gnu (RISC-V 64-bit, VisionFive 2, etc.)
    Riscv64Linux,
    /// x86_64-unknown-linux-gnu (default, server-class edge hardware)
    #[default]
    X86_64Linux,
}

impl std::fmt::Display for EdgeTarget {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.rustc_target_triple())
    }
}

impl EdgeTarget {
    /// The canonical Rust target triple for this target.
    pub fn rustc_target_triple(self) -> &'static str {
        match self {
            EdgeTarget::Aarch64Linux  => "aarch64-unknown-linux-gnu",
            EdgeTarget::Armv7Linux    => "armv7-unknown-linux-gnueabihf",
            EdgeTarget::Riscv64Linux  => "riscv64gc-unknown-linux-gnu",
            EdgeTarget::X86_64Linux   => "x86_64-unknown-linux-gnu",
        }
    }

    /// Returns cargo CLI arguments for cross-compiling to this target.
    ///
    /// Example output for `Aarch64Linux`:
    /// ```text
    /// ["--target", "aarch64-unknown-linux-gnu"]
    /// ```
    pub fn cargo_build_args(self) -> Vec<&'static str> {
        match self {
            EdgeTarget::X86_64Linux => vec![],  // native — no --target needed
            _ => vec!["--target", self.rustc_target_triple()],
        }
    }

    /// Linker prefix used by the GNU cross-toolchain for this target.
    pub fn linker_prefix(self) -> Option<&'static str> {
        match self {
            EdgeTarget::Aarch64Linux  => Some("aarch64-linux-gnu-gcc"),
            EdgeTarget::Armv7Linux    => Some("arm-linux-gnueabihf-gcc"),
            EdgeTarget::Riscv64Linux  => Some("riscv64-linux-gnu-gcc"),
            EdgeTarget::X86_64Linux   => None,
        }
    }

    /// Whether this target requires a cross-compiler toolchain.
    pub fn is_cross(self) -> bool {
        !matches!(self, EdgeTarget::X86_64Linux)
    }

    /// Returns all supported targets.
    pub fn all() -> &'static [EdgeTarget] {
        &[
            EdgeTarget::X86_64Linux,
            EdgeTarget::Aarch64Linux,
            EdgeTarget::Armv7Linux,
            EdgeTarget::Riscv64Linux,
        ]
    }
}
