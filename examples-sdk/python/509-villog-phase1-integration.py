#!/usr/bin/env python3
"""509-villog-phase1-integration — Python SDK equivalent
Compile: vil compile --from python --input 509-villog-phase1-integration.py --release
"""
import os
from vil import VilPipeline, VilServer, ServiceProcess

server = VilServer("app", port=8080)
server.compile()
