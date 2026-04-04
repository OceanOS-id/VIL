#!/usr/bin/env python3
"""002-basic-vilapp-gateway — Python SDK equivalent
Compile: vil compile --from python --input 002-basic-vilapp-gateway.py --release
"""
import os
from vil import VilPipeline, VilServer, ServiceProcess

server = VilServer("vil-app-gateway", port=3081)
gw = server.service_process("gw")
gw.endpoint("POST", "/trigger", "trigger_handler")
server.compile()
