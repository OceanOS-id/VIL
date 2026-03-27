# vil_edge_deploy

VIL Edge Deployment — ARM/RISC-V build profiles and minimal runtime configuration.

## Overview

`vil_edge_deploy` provides structured configuration and build helpers for
deploying VIL processes on resource-constrained edge and IoT hardware. It
supports aarch64 (ARM64), armv7 (ARM32), riscv64gc (RISC-V), and x86_64 targets
with three preset profiles (Minimal, Standard, Full).

## Supported Targets

| Target             | Triple                             | Cross |
|--------------------|------------------------------------|-------|
| `X86_64Linux`      | x86_64-unknown-linux-gnu           | No    |
| `Aarch64Linux`     | aarch64-unknown-linux-gnu          | Yes   |
| `Armv7Linux`       | armv7-unknown-linux-gnueabihf      | Yes   |
| `Riscv64Linux`     | riscv64gc-unknown-linux-gnu        | Yes   |

## Profiles

| Profile    | SHM      | Max Processes | Scheduler    |
|------------|----------|---------------|--------------|
| `Minimal`  | 4 MB     | 16            | SingleCore   |
| `Standard` | 64 MB    | 64            | MultiCore    |
| `Full`     | 256 MB   | 256           | MultiCore    |

## Quick Start

```rust,ignore
use vil_edge_deploy::{process, EdgeTarget, EdgeProfile};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Build a config for ARM64 with the Standard profile.
    let config = process::create(EdgeTarget::Aarch64Linux, EdgeProfile::Standard)?;

    // Get cargo build arguments for cross-compilation.
    let args = config.target.cargo_build_args();
    // args == ["--target", "aarch64-unknown-linux-gnu"]

    // Get the linker prefix for .cargo/config.toml.
    let linker = config.target.linker_prefix();
    // linker == Some("aarch64-linux-gnu-gcc")

    // Serialize config to YAML for deployment manifests.
    let yaml = config.to_yaml()?;
    println!("{}", yaml);

    Ok(())
}
```

## YAML Config

```yaml
target: aarch64_linux
profile: standard
shm_size_kb: 65536
max_processes: 64
scheduler_mode: multi_core
offline_buffer_kb: 8192
```

## Compliance

- Uses `vil_log` only — no `println!` or `tracing` calls.
- Fault codes are plain enum variants hashed via `register_str()`.
- `process.rs` exposes the `create()` constructor function.
