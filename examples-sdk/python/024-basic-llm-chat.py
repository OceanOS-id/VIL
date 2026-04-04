#!/usr/bin/env python3
"""024-basic-llm-chat — Python SDK equivalent
Compile: vil compile --from python --input 024-basic-llm-chat.py --release
"""
import os
from vil import VilPipeline, VilServer, ServiceProcess

server = VilServer("llm-chat", port=8080)
chat = server.service_process("chat")
chat.endpoint("POST", "/chat", "chat_handler")
server.compile()
