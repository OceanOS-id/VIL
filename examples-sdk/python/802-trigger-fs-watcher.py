#!/usr/bin/env python3
"""802-trigger-fs-watcher — Python SDK equivalent
Compile: vil compile --from python --input 802-trigger-fs-watcher.py --release
"""
import os
from vil import VilPipeline, VilServer, ServiceProcess

server = VilServer("app", port=8080)
server.compile()
