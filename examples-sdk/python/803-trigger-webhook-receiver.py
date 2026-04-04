#!/usr/bin/env python3
"""803-trigger-webhook-receiver — Python SDK equivalent
Compile: vil compile --from python --input 803-trigger-webhook-receiver.py --release
"""
import os
from vil import VilPipeline, VilServer, ServiceProcess

server = VilServer("app", port=8080)
server.compile()
