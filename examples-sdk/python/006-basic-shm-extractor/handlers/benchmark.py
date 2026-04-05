"""GET /benchmark — nanosecond timestamp benchmark.

Pure business logic — no VIL SDK dependency.
"""
import time


def handle_benchmark(body: bytes) -> dict:
    return {
        "ok": True,
        "timestamp_ns": time.time_ns(),
    }
