#!/usr/bin/env python3
"""
VIL Sidecar Example -- Fraud Detection Service
==================================================

Python process running as a VIL sidecar, receiving invoke requests
from the Rust vil-server host via UDS.

Run:
  python3 fraud_detector.py

Listens on /tmp/vil_sidecar_fraud.sock and handles:
  - score: Returns a risk score (0.0 - 1.0)
  - validate: Validates customer data against fraud rules
"""

from vil_sidecar import VlangSidecar
import random

sidecar = VlangSidecar("fraud", version="1.0")


@sidecar.method("score")
def score_transaction(data: dict) -> dict:
    """Score a transaction for fraud risk."""
    risk_score = round(random.uniform(0.0, 1.0), 4)
    action = "block" if risk_score > 0.8 else "flag" if risk_score > 0.5 else "allow"

    return {
        "risk_score": risk_score,
        "action": action,
        "model": "fraud-v2.1",
        "features_used": ["amount", "velocity", "geo_distance"],
    }


@sidecar.method("validate")
def validate_customer(data: dict) -> dict:
    """Validate customer data against fraud rules."""
    return {
        "valid": True,
        "checks_passed": ["identity", "sanctions", "pep"],
        "checks_failed": [],
    }


if __name__ == "__main__":
    sidecar.run()
