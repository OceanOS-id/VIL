#!/usr/bin/env python3
"""027-basic-vilserver-minimal — Python SDK equivalent
Compile: vil compile --from python --input 027-basic-vilserver-minimal.py --release
"""
import os
from vil import VilPipeline, VilServer, ServiceProcess

server = VilServer("app", port=8080)
server.compile()
