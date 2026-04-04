#!/usr/bin/env python3
"""604-db-elastic-search — Python SDK equivalent
Compile: vil compile --from python --input 604-db-elastic-search.py --release
"""
import os
from vil import VilPipeline, VilServer, ServiceProcess

server = VilServer("app", port=8080)
server.compile()
