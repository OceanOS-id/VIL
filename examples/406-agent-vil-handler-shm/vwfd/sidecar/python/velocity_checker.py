#!/usr/bin/env python3
"""Transaction Velocity Checker — Python Sidecar"""
import json, sys

def check_velocity(data):
    if isinstance(data, str):
        data = json.loads(data) if data.strip() else {}
    recent_count = data.get("recent_tx_count", data.get("transactions_last_hour", 0))
    time_window_hours = data.get("time_window_hours", 1)
    rate = recent_count / max(time_window_hours, 0.01)
    score = min(95, int(rate * 8)) if rate > 1 else 10
    return {"score": score, "tx_per_hour": round(rate, 2), "risk": "HIGH" if score > 70 else "NORMAL"}

if __name__ == "__main__":
    for line in sys.stdin:
        line = line.strip()
        if not line:
            continue
        try:
            data = json.loads(line)
            result = check_velocity(data)
            print(json.dumps(result), flush=True)
        except Exception as e:
            print(json.dumps({"error": str(e)}), flush=True)
