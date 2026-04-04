#!/usr/bin/env python3
"""017-basic-production-fullstack — Python SDK equivalent
Compile: vil compile --from python --input 017-basic-production-fullstack.py --release
"""
import os
from vil import VilPipeline, VilServer, ServiceProcess

server = VilServer("production-fullstack", port=8080)
fullstack = server.service_process("fullstack")
fullstack.endpoint("GET", "/stack", "stack_info")
fullstack.endpoint("GET", "/config", "full_config")
fullstack.endpoint("GET", "/sprints", "sprints")
fullstack.endpoint("GET", "/middleware", "middleware_info")
admin = server.service_process("admin")
admin.endpoint("GET", "/config", "full_config")
root = server.service_process("root")
server.compile()
