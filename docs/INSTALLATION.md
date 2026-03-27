# VIL Installation Guide

This guide covers installation and setup of VIL for different platforms and use cases.

## Prerequisites

### System Requirements
- **OS**: Linux (primary), macOS, Windows (experimental)
- **Rust**: 1.70 or later (MSRV)
- **RAM**: 4GB minimum (8GB+ recommended for large pipelines)
- **CPU**: Multi-core processor recommended

### Required Dependencies

#### Linux
```bash
# Ubuntu/Debian
sudo apt-get update
sudo apt-get install -y \
    build-essential \
    curl \
    pkg-config \
    libssl-dev \
    libc6-dev

# RHEL/CentOS/Fedora
sudo dnf install -y \
    gcc \
    gcc-c++ \
    make \
    curl \
    openssl-devel \
    glibc-devel

# Arch
sudo pacman -S base-devel curl openssl
```

#### macOS
```bash
# Install Homebrew if not already installed
/bin/bash -c "$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)"

# Install dependencies
brew install pkg-config openssl
```

#### Windows
```powershell
# Using chocolatey
choco install rust-ms visualstudio-buildtools -y

# Or download Rust from https://rustup.rs/
```

### Install Rust (if not already installed)
```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env

# Verify installation
rustc --version
cargo --version
```

---

## Installation Methods

### Method 1: Clone from GitHub (Recommended for Development)

```bash
# Clone the repository
git clone https://github.com/OceanOS-id/VIL.git
cd vil

# Verify the workspace structure
ls -la

# Expected structure:
# - crates/             → VIL runtime and compiler crates
# - examples/           → Example pipelines and demos
# - docs/              → Documentation
# - Cargo.toml          → Workspace manifest
# - README.md           → This file's sibling
```

### Method 2: Add as Dependency (For Using VIL in Your Project)

In your `Cargo.toml`:
```toml
[dependencies]
vil_rt = { git = "https://github.com/OceanOS-id/VIL", rev = "main" }
vil_types = { git = "https://github.com/OceanOS-id/VIL", rev = "main" }
vil_macros = { git = "https://github.com/OceanOS-id/VIL", rev = "main" }
```

Or for published crates (when available on crates.io):
```toml
[dependencies]
vil_rt = "2.0"
vil_types = "2.0"
vil_macros = "2.0"
```

---

## Build from Source

### Quick Build
```bash
cd vil
cargo build --workspace --release
```

### Build with Specific Features

```bash
# With observability features enabled (default)
cargo build --workspace --release --features "observability"

# With FFI/VAPI support
cargo build --workspace --release --features "vapi,ffi"

# Minimal build (core runtime only)
cargo build --workspace --release --no-default-features
```

### Build Crate-by-Crate

```bash
# Build just the runtime
cargo build --release -p vil_rt

# Build with macros
cargo build --release -p vil_macros

# Build validation layer
cargo build --release -p vil_validate

# Build IR codegen
cargo build --release -p vil_codegen_rust
```

---

## Verification

### Run Tests
```bash
# Full test suite
cargo test --workspace --release

# Test specific crate
cargo test -p vil_rt --release

# Test with output
cargo test --workspace --release -- --nocapture

# Run ignored tests
cargo test --workspace --release -- --ignored
```

### Run Examples
```bash
# Semantic types demo
cargo run --example semantic_types_demo --release

# Camera pipeline with observability
cargo run --example camera_pipeline --release

# Distributed topology simulation
cargo run --example distributed_topo_demo --release

# Full v2 feature showcase
cargo run --example vil_v2_full_demo --release

# Lifecycle DSL demo
cargo run --example lifecycle_dsl_demo --release

# Fault tolerance demo
cargo run --example fault_tolerance_demo --release

# Memory class demo
cargo run --example memory_class_demo --release

# Trust zone demo
cargo run --example trust_zone_demo --release

# Execution contract export
cargo run --example execution_contract_demo --release

# HTTP webhook integration
cargo run --example webhook_pipeline --release
```

### Generate Documentation
```bash
# Build and open API documentation
cargo doc --workspace --no-deps --open

# Build docs without opening
cargo doc --workspace --no-deps

# Generated docs location: target/doc/vil_rt/index.html
```

---

## Platform-Specific Setup

### Linux Setup

#### Ubuntu 22.04 LTS (Recommended)
```bash
# Install system dependencies
sudo apt-get update
sudo apt-get install -y \
    build-essential \
    cmake \
    pkg-config \
    libssl-dev \
    libc6-dev

# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env

# Clone and build VIL
git clone https://github.com/OceanOS-id/VIL.git
cd vil
cargo build --workspace --release

# Run tests to verify
cargo test --workspace --release
```

#### RHEL/CentOS 8+
```bash
# Enable EPEL and PowerTools
sudo dnf install -y epel-release
sudo dnf config-manager --set-enabled powertools

# Install dependencies
sudo dnf install -y \
    gcc \
    gcc-c++ \
    make \
    cmake \
    pkg-config \
    openssl-devel

# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env

# Build VIL
git clone https://github.com/OceanOS-id/VIL.git
cd vil
cargo build --workspace --release
```

### macOS Setup

```bash
# Install Homebrew
/bin/bash -c "$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)"

# Install dependencies
brew install pkg-config openssl cmake

# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env

# Clone and build
git clone https://github.com/OceanOS-id/VIL.git
cd vil
cargo build --workspace --release

# On Apple Silicon (M1/M2/M3), Rust should auto-detect arm64
# If not, explicitly set target:
rustup target add aarch64-apple-darwin
cargo build --workspace --release --target aarch64-apple-darwin
```

### Windows Setup

```powershell
# Install Rust via rustup
Invoke-WebRequest https://win.rustup.rs -OutFile rustup-init.exe
.\rustup-init.exe

# Or use Chocolatey
choco install rust-ms -y

# Open new PowerShell and verify
rustc --version
cargo --version

# Clone and build
git clone https://github.com/OceanOS-id/VIL.git
cd vil
cargo build --workspace --release

# Run tests
cargo test --workspace --release
```

---

## Docker Setup (Optional)

### Build Docker Image
```dockerfile
FROM rust:1.70-bookworm

RUN apt-get update && apt-get install -y \
    build-essential \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /vil

COPY . .

RUN cargo build --workspace --release

ENTRYPOINT ["cargo", "run", "--release"]
```

Build and run:
```bash
docker build -t vil:latest .
docker run --rm vil:latest --example camera_pipeline
```

---

## Development Environment Setup

### Editor Setup

#### VS Code
```json
// .vscode/settings.json
{
    "rust-analyzer.checkOnSave.command": "clippy",
    "rust-analyzer.checkOnSave.extraArgs": ["--all-targets"],
    "[rust]": {
        "editor.formatOnSave": true,
        "editor.defaultFormatter": "rust-lang.rust-analyzer"
    }
}
```

#### CLion/IntelliJ IDEA
1. Install Rust plugin from marketplace
2. File → Project Structure → SDK → Add Rust SDK
3. Configure as managed (auto-download)

### Git Setup
```bash
# Clone with SSH (requires SSH key setup)
git clone git@github.com:OceanOS-id/VIL.git

# Or with HTTPS
git clone https://github.com/OceanOS-id/VIL.git

# Set up pre-commit hooks (if available)
cd vil
git config core.hooksPath .githooks
chmod +x .githooks/*
```

---

## Troubleshooting

### Common Issues

#### Issue: "cargo not found"
```bash
# Solution: Add Cargo to PATH
source $HOME/.cargo/env

# Or add to ~/.bashrc or ~/.zshrc:
export PATH="$HOME/.cargo/bin:$PATH"
```

#### Issue: "SHM allocation failed"
```bash
# Check available shared memory
df -h /dev/shm

# If insufficient, increase (Linux):
sudo mount -o remount,size=4G /dev/shm
```

#### Issue: "Permission denied" on Linux
```bash
# Ensure user is in correct group
sudo usermod -aG users $USER
newgrp users

# Check /dev/shm permissions
ls -ld /dev/shm
chmod 1777 /dev/shm
```

#### Issue: "Compilation fails on macOS"
```bash
# Update Xcode command line tools
xcode-select --install

# Or reset installation
sudo rm -rf /Library/Developer/CommandLineTools
xcode-select --install
```

#### Issue: "Cannot allocate memory"
```bash
# Increase ulimits
ulimit -a  # Check current limits

# Temporary increase
ulimit -Hn 1048576  # Hard limit
ulimit -Sn 65536    # Soft limit

# Permanent (add to ~/.bashrc or ~/.zshrc):
ulimit -Sn 65536
ulimit -Hn 1048576
```

### Getting Help

1. **Check logs**:
   ```bash
   cargo build --workspace 2>&1 | tee build.log
   ```

2. **Enable verbose output**:
   ```bash
   RUST_LOG=debug cargo build --workspace
   ```

3. **Check system info**:
   ```bash
   uname -a
   rustc --version
   cargo --version
   ldd --version  # Linux
   ```

4. **Report issues** on [GitHub Issues](https://github.com/OceanOS-id/VIL/issues) with:
   - OS and version
   - Rust version (`rustc --version`)
   - Output of `cargo --version`
   - Full error message and build log

---

## Next Steps

After successful installation:

1. **Run examples**: See [Quick Start](./QUICK_START.md)
2. **Read the guide**: See [Developer Guide](./vil/VIL-Developer-Guide.md)
3. **Explore crates**: See [API Documentation](./target/doc/vil_rt/index.html)
4. **Contribute**: See [Contributing Guidelines](./CONTRIBUTING.md)

---

## vil-server Quick Setup

```bash
# Scaffold a new server project
vil server new my-api
cd my-api
cargo run
# → http://localhost:8080
```

Or add to an existing Cargo.toml:
```toml
[dependencies]
vil_server = { git = "https://github.com/OceanOS-id/VIL.git" }
tokio = { version = "1", features = ["full"] }
serde = { version = "1.0", features = ["derive"] }
```

See [Getting Started with vil-server](./tutorials/tutorial-getting-started-server.md) for full tutorial.

---

## Additional Resources

- **Repository**: https://github.com/OceanOS-id/VIL
- **vil-server Guide**: [vil-server-guide.md](./vil-server/vil-server-guide.md)
- **API Reference**: [API-REFERENCE-SERVER.md](./vil-server/API-REFERENCE-SERVER.md)
- **Examples**: [examples/](../examples/)
- **API Docs**: Run `cargo doc --open --no-deps`

---

**Last Updated**: 2026-03-18 | **Verified on**: Ubuntu 24.04, Rust 1.75