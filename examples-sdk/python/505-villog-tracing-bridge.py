#!/usr/bin/env python3
"""505-villog-tracing-bridge — Python SDK equivalent
Compile: vil compile --from python --input 505-villog-tracing-bridge.py --release
"""
import os
from vil import VilPipeline, VilServer, ServiceProcess

server = VilServer("app", port=8080)
server.compile()
