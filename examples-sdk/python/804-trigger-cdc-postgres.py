#!/usr/bin/env python3
"""804-trigger-cdc-postgres — Python SDK equivalent
Compile: vil compile --from python --input 804-trigger-cdc-postgres.py --release
"""
import os
from vil import VilPipeline, VilServer, ServiceProcess

server = VilServer("app", port=8080)
server.compile()
