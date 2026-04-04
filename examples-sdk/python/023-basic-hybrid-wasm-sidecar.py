#!/usr/bin/env python3
"""023-basic-hybrid-wasm-sidecar — Python SDK equivalent
Compile: vil compile --from python --input 023-basic-hybrid-wasm-sidecar.py --release
"""
import os
from vil import VilPipeline, VilServer, ServiceProcess

server = VilServer("hybrid-pipeline", port=8080)
pipeline = server.service_process("pipeline")
pipeline.endpoint("GET", "/", "index")
pipeline.endpoint("POST", "/validate", "validate_order")
pipeline.endpoint("POST", "/price", "calculate_price")
pipeline.endpoint("POST", "/fraud", "fraud_check")
pipeline.endpoint("POST", "/order", "process_order")
server.compile()
