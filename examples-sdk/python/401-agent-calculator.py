#!/usr/bin/env python3
"""401-agent-calculator — Python SDK equivalent
Compile: vil compile --from python --input 401-agent-calculator.py --release
"""
import os
from vil import VilPipeline, VilServer, ServiceProcess

server = VilServer("calculator-agent", port=3120)
calc_agent = server.service_process("calc-agent")
calc_agent.endpoint("POST", "/calc", "calc_handler")
server.compile()
