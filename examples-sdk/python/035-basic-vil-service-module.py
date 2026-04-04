#!/usr/bin/env python3
"""035-basic-vil-service-module — Python SDK equivalent
Compile: vil compile --from python --input 035-basic-vil-service-module.py --release
"""
import os
from vil import VilPipeline, VilServer, ServiceProcess

server = VilServer("app", port=8080)
server.compile()
