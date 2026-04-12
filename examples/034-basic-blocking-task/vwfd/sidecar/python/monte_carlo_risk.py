#!/usr/bin/env python3
"""Monte Carlo Credit Risk Simulation — Python Sidecar (VIL SDK UDS+SHM)"""
import json, sys, os, random

# Try VIL SDK (UDS+SHM), fallback to stdin/stdout line-delimited
try:
    sys.path.insert(0, os.path.join(os.path.dirname(__file__), '../../../../crates/vil_sidecar/sdk'))
    from vil_sidecar_sdk import SidecarApp
    VIL_SDK = True
except ImportError:
    VIL_SDK = False

def simulate_risk(input_data):
    if isinstance(input_data, dict):
        data = input_data
    else:
        data = {}
    principal = data.get("principal", 100_000_000)
    default_prob = data.get("default_probability", 0.05)
    recovery_rate = data.get("recovery_rate", 0.4)
    simulations = data.get("simulations", 10_000)

    losses = []
    for _ in range(simulations):
        if random.random() < default_prob:
            loss = principal * (1 - recovery_rate) * random.uniform(0.5, 1.5)
        else:
            loss = 0
        losses.append(loss)

    losses.sort()
    avg_loss = sum(losses) / len(losses)
    var_95 = losses[int(0.95 * len(losses))]
    var_99 = losses[int(0.99 * len(losses))]
    max_loss = losses[-1]

    return {
        "expected_loss": round(avg_loss, 2),
        "var_95": round(var_95, 2),
        "var_99": round(var_99, 2),
        "max_loss": round(max_loss, 2),
        "simulations": simulations,
        "default_probability": default_prob
    }

if VIL_SDK and os.environ.get("VIL_SIDECAR_SOCKET"):
    # UDS+SHM mode
    app = SidecarApp("monte_carlo_risk")
    app.handler("execute")(simulate_risk)
    app.run()
else:
    # Stdin/stdout line-delimited JSON (fallback)
    for line in sys.stdin:
        line = line.strip()
        if not line:
            continue
        try:
            data = json.loads(line)
            result = simulate_risk(data)
            print(json.dumps(result), flush=True)
        except Exception as e:
            print(json.dumps({"error": str(e)}), flush=True)
