#!/usr/bin/env python3
"""Handler: POST /transform — uppercase data, double numeric value.

Business logic (mirrors Rust):
1. Parse JSON body: {data: string, value: float}
2. Transform: uppercase data, double value
3. Add timestamp
"""
import sys
import json
import time

body = sys.stdin.buffer.read()
try:
    req = json.loads(body)
except json.JSONDecodeError:
    req = {"data": "", "value": 0.0}

data = req.get("data", "")
value = req.get("value", 0.0)
timestamp = int(time.time())

print(json.dumps({
    "transformed": data.upper(),
    "original": data,
    "value_doubled": value * 2.0,
    "timestamp": timestamp,
}))
