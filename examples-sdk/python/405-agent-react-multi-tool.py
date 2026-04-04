#!/usr/bin/env python3
"""405-agent-react-multi-tool — Python SDK equivalent
Compile: vil compile --from python --input 405-agent-react-multi-tool.py --release
"""
import os
from vil import VilPipeline, VilServer, ServiceProcess

server = VilServer("react-multi-tool-agent", port=3124)
react_agent = server.service_process("react-agent")
react_agent.endpoint("POST", "/react", "react_handler")
server.compile()
