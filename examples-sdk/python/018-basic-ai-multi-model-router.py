#!/usr/bin/env python3
"""018-basic-ai-multi-model-router — Python SDK equivalent
Compile: vil compile --from python --input 018-basic-ai-multi-model-router.py --release
"""
import os
from vil import VilPipeline, VilServer, ServiceProcess

server = VilServer("ai-multi-model-router", port=3085)
router = server.service_process("router")
router.endpoint("POST", "/route", "route_handler")
server.compile()
