#!/usr/bin/env python3
"""026-basic-ai-agent — Python SDK equivalent
Compile: vil compile --from python --input 026-basic-ai-agent.py --release
"""
import os
from vil import VilPipeline, VilServer, ServiceProcess

server = VilServer("ai-agent", port=8080)
agent = server.service_process("agent")
agent.endpoint("POST", "/agent", "agent_handler")
server.compile()
