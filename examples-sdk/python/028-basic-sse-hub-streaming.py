#!/usr/bin/env python3
"""028-basic-sse-hub-streaming — Python SDK equivalent
Compile: vil compile --from python --input 028-basic-sse-hub-streaming.py --release
"""
import os
from vil import VilPipeline, VilServer, ServiceProcess

server = VilServer("sse-hub-demo", port=8080)
events = server.service_process("events")
events.endpoint("POST", "/publish", "publish")
events.endpoint("GET", "/stream", "stream")
events.endpoint("GET", "/stats", "stats")
server.compile()
