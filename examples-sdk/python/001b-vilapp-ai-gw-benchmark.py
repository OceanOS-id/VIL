#!/usr/bin/env python3
"""001b-vilapp-ai-gw-benchmark — Python SDK equivalent
Compile: vil compile --from python --input 001b-vilapp-ai-gw-benchmark.py --release
"""
import os
from vil import VilPipeline, VilServer, ServiceProcess

server = VilServer("ai-gw-bench", port=3081)
gw = server.service_process("gw")
server.compile()
