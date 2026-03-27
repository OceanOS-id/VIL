#!/usr/bin/env python3
"""
Fraud Checker Sidecar — Python ML sidecar example.

This sidecar connects to a VIL host and provides fraud scoring
via a simple rule-based model (replace with scikit-learn/etc. in production).

Architecture:
  VlangApp (Rust) → UDS + SHM → fraud_checker.py (Python)
  - Zero-copy: request/response data in /dev/shm
  - Transport: Unix Domain Socket (descriptors only)

Usage:
  # The VIL host starts first, then this sidecar connects:
  python fraud_checker.py

  # Or auto-spawned by VlangApp with:
  #   .sidecar(SidecarConfig::new("fraud-checker")
  #       .command("python examples-sdk/sidecar/python/fraud_checker.py"))
"""

import sys
import os

# Add SDK to path
sys.path.insert(0, os.path.join(os.path.dirname(__file__), '../../../sdk/sidecar/python'))

from vil_sidecar import VlangSidecar

app = VlangSidecar("fraud-checker", version="1.0.0")


@app.handler("fraud_check")
def fraud_check(request: dict) -> dict:
    """
    Score a transaction for fraud risk.

    Input:  {"amount": float, "merchant_category": str, "country": str}
    Output: {"score": float, "is_fraud": bool, "reason": str}
    """
    amount = request.get("amount", 0)
    category = request.get("merchant_category", "unknown")
    country = request.get("country", "unknown")

    # Simple rule-based scoring (replace with ML model in production)
    score = 0.0

    # High amount
    if amount > 10000:
        score += 0.4
    elif amount > 5000:
        score += 0.2

    # Risky categories
    risky_categories = {"gambling", "crypto", "adult", "money_transfer"}
    if category in risky_categories:
        score += 0.3

    # Risky countries
    risky_countries = {"XX", "YY", "ZZ"}
    if country in risky_countries:
        score += 0.2

    # Clamp to [0, 1]
    score = min(score, 1.0)

    reason = "clean"
    if score > 0.8:
        reason = "high_risk_transaction"
    elif score > 0.5:
        reason = "moderate_risk"

    return {
        "score": round(score, 3),
        "is_fraud": score > 0.8,
        "reason": reason,
        "model_version": "rules-v1.0",
    }


@app.handler("batch_score")
def batch_score(request: dict) -> dict:
    """
    Score multiple transactions in a batch.

    Input:  {"transactions": [{"amount": ..., ...}, ...]}
    Output: {"results": [{"score": ..., ...}, ...]}
    """
    transactions = request.get("transactions", [])
    results = [fraud_check(tx) for tx in transactions]
    return {"results": results, "batch_size": len(results)}


if __name__ == "__main__":
    print("=" * 50)
    print("  Fraud Checker Sidecar (Python)")
    print("  VIL WASM FaaS + Sidecar Hybrid")
    print("=" * 50)
    print()
    app.run()
