#!/usr/bin/env python3
"""027 — VilServer Minimal (No VX)
Equivalent to: examples/027-basic-vilserver-minimal (Rust)
Compile: vil compile --from python --input 027-basic-vilserver-minimal.py --release
"""
import os
from vil import VilServer

server = VilServer("minimal-api", port=8080)

# -- Fault type ---------------------------------------------------------------
server.fault("ApiFault", variants=["InvalidInput", "NotFound"])

# -- Routes (no ServiceProcess, no VX) ----------------------------------------
server.get("/hello", handler="hello")
server.post("/echo", handler="echo")

# Built-in: GET /health, /ready, /metrics, /info

# -- Emit / compile -----------------------------------------------------------
if os.environ.get("VIL_COMPILE_MODE") == "manifest":
    print(server.to_yaml())
else:
    server.compile()
