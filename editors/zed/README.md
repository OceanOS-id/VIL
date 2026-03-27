# VIL Language Server — Zed Setup

## Prerequisites

Build the LSP binary:

```bash
cd /path/to/vil
cargo build --release -p vil_lsp
# Binary: target/release/vil-lsp
```

Ensure `vil-lsp` is in your `PATH`:

```bash
# Option 1: symlink
sudo ln -s /path/to/vil/target/release/vil-lsp /usr/local/bin/vil-lsp

# Option 2: add to PATH
export PATH="/path/to/vil/target/release:$PATH"
```

## Configuration

Add to your Zed settings (`~/.config/zed/settings.json`):

```json
{
  "lsp": {
    "vil-lsp": {
      "binary": {
        "path": "vil-lsp",
        "arguments": []
      },
      "languages": ["Rust"]
    }
  },
  "languages": {
    "Rust": {
      "language_servers": ["rust-analyzer", "vil-lsp"]
    }
  }
}
```

Or if using full path:

```json
{
  "lsp": {
    "vil-lsp": {
      "binary": {
        "path": "/path/to/vil/target/release/vil-lsp",
        "arguments": []
      },
      "languages": ["Rust"]
    }
  }
}
```

## Features

- **Diagnostics** — Warns about incorrect VIL macro usage (`#[vil_state]`, `#[vil_event]`, `#[vil_handler]`)
- **Completions** — Suggests VIL macros, derive attributes, and method signatures
- **Hover** — Documentation on hover for VIL types and macros

## Troubleshooting

```bash
# Check if vil-lsp is in PATH
which vil-lsp

# Test manually (should wait for JSON-RPC on stdin)
vil-lsp

# Enable debug logging
RUST_LOG=debug vil-lsp
```
