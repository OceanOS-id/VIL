#!/usr/bin/env python3
"""501-villog-stdout-dev — Python SDK equivalent
Compile: vil compile --from python --input 501-villog-stdout-dev.py --release
"""
import os
from vil import VilPipeline, VilServer, ServiceProcess

server = VilServer("app", port=8080)
server.compile()
