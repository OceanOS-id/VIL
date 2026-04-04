#!/usr/bin/env python3
"""039-basic-observer-dashboard — Python SDK equivalent
Compile: vil compile --from python --input 039-basic-observer-dashboard.py --release
"""
import os
from vil import VilPipeline, VilServer, ServiceProcess

server = VilServer("observer-demo", port=8080)
demo = server.service_process("demo")
demo.endpoint("GET", "/hello", "hello")
demo.endpoint("POST", "/echo", "echo")
server.compile()
