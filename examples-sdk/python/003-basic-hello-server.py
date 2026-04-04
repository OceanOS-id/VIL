#!/usr/bin/env python3
"""003-basic-hello-server — Python SDK equivalent
Compile: vil compile --from python --input 003-basic-hello-server.py --release
"""
import os
from vil import VilPipeline, VilServer, ServiceProcess

server = VilServer("vil-basic-hello-server", port=8080)
gw = server.service_process("gw")
gw.endpoint("POST", "/transform", "transform")
gw.endpoint("POST", "/echo", "echo")
gw.endpoint("GET", "/health", "health")
server.compile()
