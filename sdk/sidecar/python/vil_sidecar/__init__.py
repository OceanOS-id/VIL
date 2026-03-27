"""
vil_sidecar — VIL Sidecar SDK for Python

Write VIL sidecar handlers in Python with zero-copy SHM IPC.

Usage:
    from vil_sidecar import VilSidecar

    app = VilSidecar("fraud-checker")

    @app.handler("fraud_check")
    def fraud_check(request: dict) -> dict:
        score = ml_model.predict(request["features"])
        return {"score": float(score), "is_fraud": score > 0.8}

    app.run()
"""

from vil_sidecar.sidecar import VilSidecar
from vil_sidecar.handler import vil_handler

__all__ = ["VilSidecar", "vil_handler"]
__version__ = "0.1.0"
