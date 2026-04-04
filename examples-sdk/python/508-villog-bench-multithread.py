#!/usr/bin/env python3
"""508-villog-bench-multithread — Python SDK equivalent
Compile: vil compile --from python --input 508-villog-bench-multithread.py --release
"""
import os
from vil import VilPipeline, VilServer, ServiceProcess

server = VilServer("app", port=8080)
server.compile()
