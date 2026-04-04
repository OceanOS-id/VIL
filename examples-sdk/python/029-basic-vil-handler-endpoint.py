#!/usr/bin/env python3
"""029-basic-vil-handler-endpoint — Python SDK equivalent
Compile: vil compile --from python --input 029-basic-vil-handler-endpoint.py --release
"""
import os
from vil import VilPipeline, VilServer, ServiceProcess

server = VilServer("macro-demo", port=8080)
demo = server.service_process("demo")
demo.endpoint("GET", "/plain", "plain_handler")
demo.endpoint("GET", "/handled", "handled_handler")
demo.endpoint("POST", "/endpoint", "endpoint_handler")
server.compile()
