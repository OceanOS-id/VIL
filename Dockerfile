# =============================================================================
# VIL — Provisionable Server (VSAL — see LICENSE-VSAL)
# =============================================================================
# Starts an EMPTY server with admin API. Provision at runtime:
#   POST /api/admin/upload           — workflow YAML (auto-provisions handlers)
#   POST /api/admin/upload/plugin    — .so NativeCode handler
#   POST /api/admin/upload/wasm      — .wasm module
#   GET  /api/admin/handlers         — list registered handlers
#
# Usage:
#   docker build -t vilfounder/vil:0.4.0 .
#   docker run -p 3080:3080 vilfounder/vil:0.4.0
#   docker run -p 3080:3080 -e ADMIN_KEY=secret vilfounder/vil:0.4.0
#
# License: Vastar Source Available License (VSAL). Internal business use is
# free. Operating this image as a multi-tenant Workflow-as-a-Service (WaaS)
# requires a commercial agreement with Vastar. See LICENSING.md for details.
# =============================================================================

# ── Builder ─────────────────────────────────────────────────────────────────

FROM rust:1.93-bookworm AS builder

RUN apt-get update && apt-get install -y --no-install-recommends \
    cmake libssl-dev libsasl2-dev librdkafka-dev protobuf-compiler pkg-config \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /build

COPY Cargo.toml Cargo.lock ./
COPY crates/ crates/

# Stub out example workspace members for workspace resolution
RUN grep -oP '"examples/[^"]+' Cargo.toml | tr -d '"' | sort -u | while read d; do \
      mkdir -p "$d/src"; \
      echo 'fn main(){}' > "$d/src/main.rs"; \
      [ -f "$d/Cargo.toml" ] || printf '[package]\nname = "stub-%s"\nversion = "0.0.0"\nedition = "2021"\n' \
        "$(echo "$d" | tr '/' '-')" > "$d/Cargo.toml"; \
    done

RUN cargo build --release -p vil-server-provision

# ── Runtime ─────────────────────────────────────────────────────────────────

FROM debian:bookworm-slim

# ── OCI labels — shown on Docker Hub and by docker inspect ────────────────
LABEL org.opencontainers.image.title="VIL Provisionable Server"
LABEL org.opencontainers.image.description="VIL — Vastar Intermediate Language. Provisionable server that starts empty and accepts workflow uploads at runtime via /api/admin/*. Hot-reload in ~200ms. Mixes native Rust, WASM (4 langs), and sidecar (9 langs) handlers per workflow activity."
LABEL org.opencontainers.image.vendor="PT RAG Mid Solution (Vastar)"
LABEL org.opencontainers.image.source="https://github.com/OceanOS-id/VIL"
LABEL org.opencontainers.image.documentation="https://vastar.id/docs/vil"
LABEL org.opencontainers.image.url="https://vastar.id/products/vil"
LABEL org.opencontainers.image.version="0.4.0"
LABEL org.opencontainers.image.licenses="VSAL"
# VSAL: Internal business use + Significant Business Process = free.
# Multi-tenant Workflow-as-a-Service (WaaS) hosting = requires commercial agreement.
# Contact: legal@midsolution.id
LABEL id.vastar.license.type="source-available"
LABEL id.vastar.license.name="Vastar Source Available License"
LABEL id.vastar.license.url="https://github.com/OceanOS-id/VIL/blob/main/LICENSE-VSAL"
LABEL id.vastar.license.waas-restriction="Operating this image as a multi-tenant Workflow-as-a-Service requires a commercial agreement with Vastar. Contact legal@midsolution.id."

RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates libssl3 libsasl2-2 librdkafka1 \
    && rm -rf /var/lib/apt/lists/*

RUN groupadd -r vil && useradd -r -g vil -m vil \
    && mkdir -p /var/lib/vil/workflows \
                /var/lib/vil/plugins \
                /var/lib/vil/modules \
    && chown -R vil:vil /var/lib/vil

USER vil

COPY --from=builder /build/target/release/vil-server /usr/local/bin/vil-server

ENV PORT=3080
ENV WORKFLOWS_DIR=/var/lib/vil/workflows
ENV VIL_PLUGIN_DIR=/var/lib/vil/plugins
ENV VIL_WASM_DIR=/var/lib/vil/modules
ENV VIL_LOG=info
ENV RUST_BACKTRACE=1

EXPOSE 3080

VOLUME ["/var/lib/vil/workflows", "/var/lib/vil/plugins", "/var/lib/vil/modules"]

ENTRYPOINT ["/usr/local/bin/vil-server"]
