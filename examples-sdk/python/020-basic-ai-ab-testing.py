#!/usr/bin/env python3
"""020-basic-ai-ab-testing — Python SDK equivalent
Compile: vil compile --from python --input 020-basic-ai-ab-testing.py --release
"""
import os
from vil import VilPipeline, VilServer, ServiceProcess

server = VilServer("ai-ab-testing-gateway", port=8080)
ab = server.service_process("ab")
ab.endpoint("POST", "/infer", "infer")
ab.endpoint("GET", "/metrics", "metrics")
ab.endpoint("POST", "/config", "update_config")
root = server.service_process("root")
root.endpoint("GET", "/", "index")
server.compile()
