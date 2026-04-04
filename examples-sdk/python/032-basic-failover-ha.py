#!/usr/bin/env python3
"""032-basic-failover-ha — Python SDK equivalent
Compile: vil compile --from python --input 032-basic-failover-ha.py --release
"""
import os
from vil import VilPipeline, VilServer, ServiceProcess

server = VilServer("payment-gateway-ha", port=8080)
primary = server.service_process("primary")
primary.endpoint("GET", "/health", "primary_health")
primary.endpoint("POST", "/charge", "primary_charge")
backup = server.service_process("backup")
backup.endpoint("GET", "/health", "backup_health")
backup.endpoint("POST", "/charge", "backup_charge")
server.compile()
