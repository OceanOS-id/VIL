# VIL Language Server — Helix Setup

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

## Configuration

Add to `~/.config/helix/languages.toml`:

```toml
[[language]]
name = "rust"
language-servers = ["rust-analyzer", "vil-lsp"]

[language-server.vil-lsp]
command = "vil-lsp"
args = []
```

Or with full path:

```toml
[language-server.vil-lsp]
command = "/path/to/vil/target/release/vil-lsp"
args = []
```

## Features

- **Diagnostics** — Warns about incorrect VIL macro usage
- **Completions** — VIL macros, derive attributes, method signatures
- **Hover** — Documentation for VIL types and macros

## Verify

Open a `.rs` file in Helix and run `:lsp-workspace/diagnostics` to confirm `vil-lsp` is active alongside `rust-analyzer`.

```bash
# Check health
hx --health rust
```
