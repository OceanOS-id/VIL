#!/usr/bin/env python3
"""Transaction Velocity Checker — Python Sidecar"""
import json, sys

def check_velocity(data):
    recent_count = data.get("recent_tx_count", 0)
    time_window_hours = data.get("time_window_hours", 1)
    rate = recent_count / max(time_window_hours, 0.01)
    score = min(95, int(rate * 8)) if rate > 1 else 10
    return {"velocity_score": score, "tx_per_hour": round(rate, 2), "risk": "HIGH" if score > 70 else "NORMAL"}

if __name__ == "__main__":
    print(json.dumps(check_velocity(json.loads(sys.stdin.read() or "{}"))))
