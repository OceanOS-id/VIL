#!/usr/bin/env python3
"""402-agent-http-researcher — Python SDK equivalent
Compile: vil compile --from python --input 402-agent-http-researcher.py --release
"""
import os
from vil import VilPipeline, VilServer, ServiceProcess

server = VilServer("http-researcher-agent", port=3121)
research_agent = server.service_process("research-agent")
research_agent.endpoint("POST", "/research", "research_handler")
research_agent.endpoint("GET", "/products", "products_handler")
server.compile()
