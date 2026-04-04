#!/usr/bin/env python3
"""206-llm-decision-routing — Python SDK equivalent
Compile: vil compile --from python --input 206-llm-decision-routing.py --release
"""
import os
from vil import VilPipeline, VilServer, ServiceProcess

server = VilServer("insurance-underwriting-ai", port=3116)
underwriter = server.service_process("underwriter")
server.compile()
