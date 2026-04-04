#!/usr/bin/env python3
"""507-villog-bench-file-drain — Python SDK equivalent
Compile: vil compile --from python --input 507-villog-bench-file-drain.py --release
"""
import os
from vil import VilPipeline, VilServer, ServiceProcess

server = VilServer("app", port=8080)
server.compile()
