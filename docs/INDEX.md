# VIL Documentation

**Version:** 5.1.0
**License:** Apache-2.0
**GitHub:** https://github.com/OceanOS-id/VIL

---

## The Two Components

VIL is organized into two components with distinct roles:

| Component | Role | Binary |
|-----------|------|--------|
| **[VIL](./vil/)** | Intermediate language (macros, semantics, zero-copy runtime) | Library crates |
| **[vil-server](./vil-server/)** | Standalone compiled server (multi-service, Tri-Lane) | `cargo build` → binary |

```
VIL Source Code
    │
    └── cargo build ──────→ vil-server (standalone binary)
                              Compile-time multi-service
                              Tri-Lane SHM inter-service mesh
```

---

## Quick Start

### Just Getting Started?
1. **[Quick Start](./QUICK_START.md)** — Build your first pipeline (10 min)
2. **[Installation](./INSTALLATION.md)** — Setup for Linux, macOS, Docker
3. **[Examples](./EXAMPLES.md)** — 18 runnable examples

### VIL (Language)
- **[VIL Concept](./vil/VIL_CONCEPT.md)** — 10 immutable design principles
- **[Developer Guide](./vil/VIL-Developer-Guide.md)** — complete language reference
- **[Architecture Overview](./ARCHITECTURE_OVERVIEW.md)** — layered system design

### vil-server (Standalone)
- **[Getting Started](./tutorials/tutorial-getting-started-server.md)** — from zero to running
- **[Developer Guide](./vil-server/vil-server-guide.md)** — full feature reference
- **[Production Guide](./tutorials/tutorial-production-server.md)** — Docker, Kubernetes
- **[API Reference](./vil-server/API-REFERENCE-SERVER.md)** — per-module docs

### Tutorials
1. [Hello Pipeline](./tutorials/tutorial-01-hello-pipeline.md)
2. [Custom Nodes](./tutorials/tutorial-02-custom-nodes.md)
3. [Tri-Lane Deep Dive](./tutorials/tutorial-03-trilane.md)
4. [Production Deployment](./tutorials/tutorial-04-production.md)

### Reference
- [SDK Integration](./vil/SDK-Integration-Guide.md) — embedding VIL
- [Changelog](./CHANGELOG.md) — release history
- [Contributing](./CONTRIBUTING.md) — how to help

---

## Documentation by Audience

### Developer (Write Services)
| Start Here | Then | Deep Dive |
|-----------|------|-----------|
| [Quick Start](./QUICK_START.md) | [Examples](./EXAMPLES.md) | [Developer Guide](./vil/VIL-Developer-Guide.md) |

### Ops (Deploy Services)
| vil-server |
|-------------|
| `cargo build` → run binary |
| [Production Guide](./tutorials/tutorial-production-server.md) |

### Architect (Design Systems)
| Document | Focus |
|----------|-------|
| [VIL Concept](./vil/VIL_CONCEPT.md) | Design principles |
| [Architecture](./ARCHITECTURE_OVERVIEW.md) | System layers |
| [Design Doc](../docs-deliverables/DESIGN-VX-process-oriented-server.md) | Architecture decisions |

---

**Version:** 5.1.0 | **License:** Apache-2.0 | **Tests:** 166 | **Crates:** 22+
