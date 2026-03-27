# VIL Language Server — JetBrains (IntelliJ IDEA / CLion / RustRover) Setup

## Prerequisites

Build the LSP binary:

```bash
cd /path/to/vil
cargo build --release -p vil_lsp
# Binary: target/release/vil-lsp
```

Ensure `vil-lsp` is in your `PATH`:

```bash
sudo ln -s /path/to/vil/target/release/vil-lsp /usr/local/bin/vil-lsp
```

## Setup

JetBrains IDEs support custom LSP servers via the built-in **LSP API** (2023.2+) or the **LSP Support** plugin for older versions.

### Option 1: Built-in LSP (RustRover / IntelliJ 2023.2+)

1. Go to **Settings** → **Languages & Frameworks** → **Language Servers**
2. Click **+** (Add)
3. Configure:

| Field | Value |
|-------|-------|
| **Name** | VIL Language Server |
| **Command** | `vil-lsp` (or full path `/path/to/vil/target/release/vil-lsp`) |
| **Arguments** | _(leave empty)_ |
| **File patterns** | `*.rs` |
| **Working directory** | `$ProjectDir$` |

4. Click **OK** → **Apply**

### Option 2: LSP Support Plugin (Older Versions)

1. Install plugin: **Settings** → **Plugins** → Search "LSP4IJ" → Install
2. Restart IDE
3. Go to **Settings** → **Languages & Frameworks** → **Language Servers (LSP4IJ)**
4. Click **+** → **New Language Server**
5. Configure:

| Field | Value |
|-------|-------|
| **Server name** | vil-lsp |
| **Command** | `vil-lsp` |
| **File type mappings** | `Rust` → `*.rs` |
| **Transport** | stdio |

6. Click **OK** → **Apply**

### JSON Configuration (`.idea/vil-lsp.json`)

For sharing LSP settings via version control, create `.idea/vil-lsp.json`:

```json
{
  "name": "VIL Language Server",
  "command": "vil-lsp",
  "args": [],
  "fileTypes": ["rs"],
  "transport": "stdio",
  "initializationOptions": {}
}
```

## Features

| Feature | Description |
|---------|-------------|
| **Diagnostics** | Warnings for incorrect VIL macro usage alongside Rust plugin diagnostics |
| **Completions** | VIL macros (`#[vil_state]`, `#[vil_handler(shm)]`), derive attributes (`VilModel`) |
| **Hover** | Documentation for VIL types, ServiceCtx methods, ShmSlice API |

## How It Works

`vil-lsp` runs as a secondary language server alongside the JetBrains Rust plugin (or `rust-analyzer` in CLion). Both provide diagnostics concurrently — the Rust plugin handles type checking and borrow analysis, while `vil-lsp` handles VIL-specific macro validation.

## Troubleshooting

### Verify Binary

```bash
# Check vil-lsp is accessible
which vil-lsp
vil-lsp --version  # should output "vil-lsp 0.1.0" then wait for stdin
```

### Enable Debug Logging

Set environment variable in the IDE run configuration or globally:

```bash
export RUST_LOG=debug
```

Then check the LSP log: **Help** → **Show Log in Explorer** → search for `vil-lsp` entries.

### Common Issues

| Issue | Solution |
|-------|----------|
| "Cannot find vil-lsp" | Add binary to PATH or use full path in settings |
| No diagnostics appearing | Check file is `.rs` and contains VIL macros |
| Conflicts with rust-analyzer | No conflict — both servers handle different concerns |
