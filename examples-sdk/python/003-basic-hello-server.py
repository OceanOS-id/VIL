#!/usr/bin/env python3
"""003 — Hello Server (VX_APP)
Equivalent to: examples/003-basic-hello-server (Rust)
Compile: vil compile --from python --input 003-basic-hello-server.py --release
"""
import os
from vil import VilServer

server = VilServer("hello-server", port=8080)

# -- ServiceProcess: hello (prefix: /api/hello) -------------------------------
hello = server.service_process("hello", prefix="/api/hello")
hello.endpoint("GET", "/", "hello")
hello.endpoint("GET", "/greet/:name", "greet")
hello.endpoint("POST", "/echo", "echo")
hello.endpoint("GET", "/shm-info", "shm_info")

# -- Built-in endpoints (auto-provided) ---------------------------------------
# GET /health, /ready, /metrics, /info

# -- Emit / compile -----------------------------------------------------------
if os.environ.get("VIL_COMPILE_MODE") == "manifest":
    print(server.to_yaml())
else:
    server.compile()
