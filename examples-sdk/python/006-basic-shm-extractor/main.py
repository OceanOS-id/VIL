#!/usr/bin/env python3
"""006-basic-shm-extractor — Python SDK equivalent
Compile: vil compile --from python --input 006-basic-shm-extractor/main.py --release

Business: Processes high-frequency market data via SHM zero-copy.

VIL handles all endpoints (HTTP, routing, SHM). Custom business logic
runs as activities within each endpoint via sidecar or wasm.

Switch mode: VIL_MODE=sidecar (default) / VIL_MODE=wasm
"""
import os
import sys
import json

from handlers import handle_ingest, handle_compute, handle_shm_stats, handle_benchmark

# ── Sidecar dispatch: VIL_HANDLER=<name> → run activity + exit ──

ACTIVITIES = {
    "handle_ingest": handle_ingest,
    "handle_compute": handle_compute,
    "handle_shm_stats": handle_shm_stats,
    "handle_benchmark": handle_benchmark,
}

vil_handler = os.environ.get("VIL_HANDLER")
vil_mode = os.environ.get("VIL_MODE", "sidecar")

if vil_handler and vil_mode != "wasm":
    body = sys.stdin.buffer.read()
    fn = ACTIVITIES.get(vil_handler)
    if fn is None:
        result = {"error": "unknown activity", "name": vil_handler}
    else:
        result = fn(body)
    print(json.dumps(result))
    sys.exit(0)

# ── SDK mode: VIL handles endpoints, activities handle business logic ──

from vil import VilServer, activity, mode_from_env

mode = mode_from_env()

handle_ingest = activity(mode, protocol="shm")(handle_ingest)
handle_compute = activity(mode, protocol="shm")(handle_compute)
handle_shm_stats = activity(mode, protocol="shm")(handle_shm_stats)
handle_benchmark = activity(mode, protocol="shm")(handle_benchmark)

server = VilServer("shm-extractor-demo", port=8080)
shm_demo = server.service_process("shm-demo")

shm_demo.endpoint("POST", "/ingest", handle_ingest)
shm_demo.endpoint("POST", "/compute", handle_compute)
shm_demo.endpoint("GET", "/shm-stats", handle_shm_stats)
shm_demo.endpoint("GET", "/benchmark", handle_benchmark)

server.compile()
