#!/usr/bin/env python3
"""038-basic-vil-app-dsl — Python SDK equivalent
Compile: vil compile --from python --input 038-basic-vil-app-dsl.py --release
"""
import os
from vil import VilPipeline, VilServer, ServiceProcess

server = VilServer("app", port=8080)
server.compile()
