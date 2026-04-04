#!/usr/bin/env python3
"""034-basic-blocking-task — Python SDK equivalent
Compile: vil compile --from python --input 034-basic-blocking-task.py --release
"""
import os
from vil import VilPipeline, VilServer, ServiceProcess

server = VilServer("credit-risk-scoring-engine", port=8080)
risk_engine = server.service_process("risk-engine")
risk_engine.endpoint("POST", "/risk/assess", "assess_risk")
risk_engine.endpoint("GET", "/risk/health", "risk_health")
server.compile()
