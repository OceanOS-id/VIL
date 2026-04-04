#!/usr/bin/env python3
"""506-villog-structured-events — Python SDK equivalent
Compile: vil compile --from python --input 506-villog-structured-events.py --release
"""
import os
from vil import VilPipeline, VilServer, ServiceProcess

server = VilServer("app", port=8080)
server.compile()
