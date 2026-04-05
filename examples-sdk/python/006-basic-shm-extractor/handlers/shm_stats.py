"""GET /shm-stats — SHM region statistics.

Pure business logic — no VIL SDK dependency.
"""


def handle_shm_stats(body: bytes) -> dict:
    return {
        "shm_available": True,
        "region_count": 0,
        "regions": [],
        "note": "Regions are created on-demand by ShmSlice and ShmResponse",
    }
