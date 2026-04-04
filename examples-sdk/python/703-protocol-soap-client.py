#!/usr/bin/env python3
"""703-protocol-soap-client — Python SDK equivalent
Compile: vil compile --from python --input 703-protocol-soap-client.py --release
"""
import os
from vil import VilPipeline, VilServer, ServiceProcess

server = VilServer("app", port=8080)
server.compile()
