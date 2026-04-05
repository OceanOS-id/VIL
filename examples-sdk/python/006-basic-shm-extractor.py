#!/usr/bin/env python3
"""006-basic-shm-extractor — Python SDK equivalent
Compile: vil compile --from python --input 006-basic-shm-extractor.py --release

Business: Demonstrates VIL zero-copy SHM (Shared Memory) patterns:
  POST /ingest    — ingest data via ShmSlice, report bytes + preview + JSON validity
  POST /compute   — CPU-bound computation with blocking_with, reports hash + elapsed
  GET  /shm-stats — SHM region statistics (available, count, regions)
  GET  /benchmark — simple timestamp benchmark

This single file serves dual purpose:
  1. Default         → emit YAML manifest (SDK mode)
  2. VIL_HANDLER=X   → run as sidecar handler (runtime mode)
"""
import os
import sys
import json
import time

from vil import VilServer, sidecar_handler


# ── Handler Implementations (decorated as sidecar + shm) ────


@sidecar_handler(protocol="shm")
def handle_ingest(body: bytes) -> dict:
    """POST /ingest — ingest data, report bytes + preview + JSON validity."""
    length = len(body)

    try:
        text = body.decode("utf-8")
        preview = text[:100]
    except UnicodeDecodeError:
        preview = f"<binary {length} bytes>"

    try:
        json.loads(body)
        is_json = True
    except (json.JSONDecodeError, UnicodeDecodeError):
        is_json = False

    return {
        "status": "ingested",
        "bytes_received": length,
        "shm_region_id": "0",
        "preview": preview,
        "is_valid_json": is_json,
        "transport": "SHM zero-copy",
        "copies": "1 (kernel \u2192 SHM), then 0 for handler read",
    }


@sidecar_handler(protocol="shm")
def handle_compute(body: bytes) -> dict:
    """POST /compute — CPU-bound hash computation, reports timing."""
    try:
        req = json.loads(body)
        iterations = int(req.get("iterations", 0))
    except (json.JSONDecodeError, ValueError):
        iterations = 0
    iterations = min(iterations, 100_000_000)

    start = time.perf_counter_ns()
    hash_val = 0
    for i in range(iterations):
        hash_val = (hash_val + i * 17 + 31) & 0xFFFFFFFFFFFFFFFF
    elapsed_ns = time.perf_counter_ns() - start

    return {
        "status": "computed",
        "iterations": iterations,
        "result_hash": hash_val,
        "elapsed_ms": elapsed_ns / 1_000_000,
        "thread": "sidecar_process",
        "note": "CPU-bound work runs in sidecar process, freeing async threads for I/O",
    }


@sidecar_handler(protocol="shm")
def handle_shm_stats(body: bytes) -> dict:
    """GET /shm-stats — SHM region statistics."""
    return {
        "shm_available": True,
        "region_count": 0,
        "regions": [],
        "note": "Regions are created on-demand by ShmSlice and ShmResponse",
    }


@sidecar_handler(protocol="shm")
def handle_benchmark(body: bytes) -> dict:
    """GET /benchmark — nanosecond timestamp benchmark."""
    return {
        "ok": True,
        "timestamp_ns": time.time_ns(),
    }


# ── Sidecar dispatcher ──────────────────────────────────────

HANDLERS = {
    "handle_ingest": handle_ingest,
    "handle_compute": handle_compute,
    "handle_shm_stats": handle_shm_stats,
    "handle_benchmark": handle_benchmark,
}

if os.environ.get("VIL_HANDLER"):
    handler_name = os.environ["VIL_HANDLER"]
    body = sys.stdin.buffer.read()
    fn = HANDLERS.get(handler_name)
    if fn is None:
        result = {"error": "unknown handler", "handler": handler_name}
    else:
        result = fn(body)
    print(json.dumps(result))
    sys.exit(0)


# ── SDK mode: declare endpoints, generate YAML ──────────────

server = VilServer("shm-extractor-demo", port=8080)
shm_demo = server.service_process("shm-demo")

shm_demo.endpoint("POST", "/ingest", handle_ingest)
shm_demo.endpoint("POST", "/compute", handle_compute)
shm_demo.endpoint("GET", "/shm-stats", handle_shm_stats)
shm_demo.endpoint("GET", "/benchmark", handle_benchmark)

server.compile()
