#!/usr/bin/env python3
"""201-llm-basic-chat — Python SDK equivalent
Compile: vil compile --from python --input 201-llm-basic-chat.py --release
"""
import os
from vil import VilPipeline, VilServer, ServiceProcess

server = VilServer("llm-basic-chat", port=3100)
chat = server.service_process("chat")
chat.endpoint("POST", "/chat", "chat_handler")
server.compile()
