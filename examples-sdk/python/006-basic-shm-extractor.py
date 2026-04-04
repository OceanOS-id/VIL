#!/usr/bin/env python3
"""006-basic-shm-extractor — Python SDK equivalent
Compile: vil compile --from python --input 006-basic-shm-extractor.py --release
"""
import os
from vil import VilPipeline, VilServer, ServiceProcess

server = VilServer("shm-extractor-demo", port=8080)
shm_demo = server.service_process("shm-demo")
shm_demo.endpoint("POST", "/ingest", "ingest")
shm_demo.endpoint("POST", "/compute", "compute")
shm_demo.endpoint("GET", "/shm-stats", "shm_stats")
shm_demo.endpoint("GET", "/benchmark", "benchmark")
server.compile()
