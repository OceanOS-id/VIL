#!/usr/bin/env python3
"""022-basic-sidecar-python — Python SDK equivalent
Compile: vil compile --from python --input 022-basic-sidecar-python.py --release
"""
import os
from vil import VilPipeline, VilServer, ServiceProcess

server = VilServer("sidecar-python-example", port=8080)
fraud = server.service_process("fraud")
fraud.endpoint("GET", "/status", "fraud_status")
fraud.endpoint("POST", "/check", "fraud_check")
root = server.service_process("root")
root.endpoint("GET", "/", "index")
server.compile()
