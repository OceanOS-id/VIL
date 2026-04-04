#!/usr/bin/env python3
"""204-llm-streaming-translator — Python SDK equivalent
Compile: vil compile --from python --input 204-llm-streaming-translator.py --release
"""
import os
from vil import VilPipeline, VilServer, ServiceProcess

server = VilServer("llm-streaming-translator", port=3103)
translator = server.service_process("translator")
server.compile()
