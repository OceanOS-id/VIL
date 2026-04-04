#!/usr/bin/env python3
"""033-basic-shm-write-through — Python SDK equivalent
Compile: vil compile --from python --input 033-basic-shm-write-through.py --release
"""
import os
from vil import VilPipeline, VilServer, ServiceProcess

server = VilServer("realtime-analytics-dashboard", port=8080)
catalog = server.service_process("catalog")
catalog.endpoint("POST", "/catalog/search", "catalog_search")
catalog.endpoint("GET", "/catalog/health", "catalog_health")
server.compile()
