#!/usr/bin/env python3
"""503-villog-multi-drain — Python SDK equivalent
Compile: vil compile --from python --input 503-villog-multi-drain.py --release
"""
import os
from vil import VilPipeline, VilServer, ServiceProcess

server = VilServer("app", port=8080)
server.compile()
