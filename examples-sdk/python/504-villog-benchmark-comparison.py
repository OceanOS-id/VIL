#!/usr/bin/env python3
"""504-villog-benchmark-comparison — Python SDK equivalent
Compile: vil compile --from python --input 504-villog-benchmark-comparison.py --release
"""
import os
from vil import VilPipeline, VilServer, ServiceProcess

server = VilServer("app", port=8080)
server.compile()
