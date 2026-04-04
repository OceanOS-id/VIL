#!/usr/bin/env python3
"""406-agent-vil-handler-shm — Python SDK equivalent
Compile: vil compile --from python --input 406-agent-vil-handler-shm.py --release
"""
import os
from vil import VilPipeline, VilServer, ServiceProcess

server = VilServer("fraud-detection-agent", port=3126)
fraud_agent = server.service_process("fraud-agent")
fraud_agent.endpoint("POST", "/detect", "detect_fraud")
fraud_agent.endpoint("GET", "/health", "health")
server.compile()
