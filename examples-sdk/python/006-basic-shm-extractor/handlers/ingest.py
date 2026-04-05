"""POST /ingest — ingest data, report bytes + preview + JSON validity.

Pure business logic — no VIL SDK dependency.
"""
import json


def handle_ingest(body: bytes) -> dict:
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
