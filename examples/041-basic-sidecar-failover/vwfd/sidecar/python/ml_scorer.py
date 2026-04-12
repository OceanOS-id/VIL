#!/usr/bin/env python3
"""ML Scorer Sidecar — reads JSON from stdin, writes prediction to stdout."""
import sys
import json
import random

def predict(features):
    """Simple mock ML model — weighted random based on feature count."""
    n = len(features) if isinstance(features, dict) else 0
    base_score = 0.5 + (n * 0.05)
    noise = random.uniform(-0.1, 0.1)
    return round(min(max(base_score + noise, 0.0), 1.0), 4)

def main():
    raw = sys.stdin.read()
    try:
        input_data = json.loads(raw)
    except json.JSONDecodeError:
        input_data = {}

    operation = input_data.get("operation", "predict")

    if operation == "health":
        result = {
            "status": "healthy",
            "circuit_state": "closed",
            "sidecar": "python_ml_scorer",
            "model_version": "1.0.0",
        }
    elif operation == "predict":
        features = input_data.get("body", input_data)
        score = predict(features)
        result = {
            "prediction": score,
            "model": "python_ml_scorer",
            "features_used": len(features) if isinstance(features, dict) else 0,
            "confidence": round(score * 0.95, 4),
        }
    else:
        result = {"error": f"unknown operation: {operation}"}

    print(json.dumps(result))

if __name__ == "__main__":
    main()
