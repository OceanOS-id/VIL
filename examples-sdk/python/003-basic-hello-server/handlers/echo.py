#!/usr/bin/env python3
"""Handler: POST /echo — echo back body with byte count.

Business logic (mirrors Rust):
1. Read raw body
2. Try to parse as JSON
3. Return byte count + echoed JSON + zero_copy flag
"""
import sys
import json

body = sys.stdin.buffer.read()
length = len(body)

try:
    parsed = json.loads(body)
except (json.JSONDecodeError, UnicodeDecodeError):
    parsed = None

print(json.dumps({
    "received_bytes": length,
    "body": parsed,
    "zero_copy": True,
}))
