"""POST /compute — CPU-bound hash computation, reports timing.

Pure business logic — no VIL SDK dependency.
"""
import json
import time


def handle_compute(body: bytes) -> dict:
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
