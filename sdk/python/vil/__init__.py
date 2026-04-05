"""VIL Python SDK — Transpile DSL for building VIL pipelines and servers.

Write pipelines/servers in Python, compile to native binary with `vil compile`.
No FFI — all code transpiles to YAML manifest -> native Rust binary.

Usage:
    from vil import VilPipeline, VilServer, VilApp, ServiceProcess

    pipeline = VilPipeline("ai-gateway")
    pipeline.sink(port=3080, path="/trigger")
    pipeline.source(url="http://localhost:4545/v1/chat", format="sse")
    pipeline.compile()
    # or: vil compile --from python --input gateway.py --release
"""

from .pipeline import (
    VilPipeline,
    VilServer,
    VilApp,
    ServiceProcess,
    string,
    number,
    boolean,
    array,
    sse,
    http,
    activity,
    handler,
    mode_from_env,
    sidecar_mode,
    wasm_mode,
    sidecar,
    wasm,
    stub,
    inline,
)

__version__ = "6.0.0"
__all__ = [
    "VilPipeline",
    "VilServer",
    "VilApp",
    "ServiceProcess",
    "string",
    "number",
    "boolean",
    "array",
    "sse",
    "http",
    "activity",
    "handler",
    "mode_from_env",
    "sidecar_mode",
    "wasm_mode",
    "sidecar",
    "wasm",
    "stub",
    "inline",
]
