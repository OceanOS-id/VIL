#!/usr/bin/env python3
"""Handler: GET /health — service health check.

Business logic (mirrors Rust):
1. Check SHM availability (always true in sidecar mode)
2. Return health status
"""
import json

print(json.dumps({
    "status": "healthy",
    "service": "vil-api",
    "shm": True,
}))
