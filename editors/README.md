# VIL Editor Integration

Editor configurations for the VIL Language Server (`vil-lsp`). Provides diagnostics, completions, and hover documentation for VIL macros (`#[vil_state]`, `#[vil_handler]`, `#[vil_event]`, `VilModel`, etc.) in Rust files.

## Build the LSP Binary

```bash
cargo build --release -p vil_lsp
# Binary: target/release/vil-lsp
```

Add to PATH:

```bash
sudo ln -s $(pwd)/target/release/vil-lsp /usr/local/bin/vil-lsp
```

## Supported Editors

| Editor | Config Location | Setup |
|--------|----------------|-------|
| **VS Code** | `editors/vscode/` | Install extension, set `vil.lsp.path` |
| **Zed** | `editors/zed/` | Add to `~/.config/zed/settings.json` |
| **Helix** | `editors/helix/` | Add to `~/.config/helix/languages.toml` |
| **JetBrains** (IntelliJ/CLion/RustRover) | `editors/jetbrains/` | Settings → Language Servers → Add `vil-lsp` |

Any editor supporting LSP stdio can use `vil-lsp` — just point it at the binary.

## Features

| Feature | Description |
|---------|-------------|
| **Diagnostics** | Warns about incorrect VIL macro usage, VASI violations, missing derives |
| **Completions** | VIL macros, derive attributes (`VilModel`, `VilWsEvent`), method signatures |
| **Hover** | Documentation for VIL types, macros, and ServiceCtx methods |

## How It Works

`vil-lsp` runs alongside `rust-analyzer` as a secondary language server. It parses VIL-specific patterns (macro attributes, derive macros) and provides VIL-aware intelligence that `rust-analyzer` cannot.

```
Editor ──stdio──> vil-lsp    (VIL macros, diagnostics)
       ──stdio──> rust-analyzer  (Rust types, borrow checker)
```

Both servers run concurrently — no conflict.
