#!/usr/bin/env python3
"""010-basic-websocket-chat — Python SDK equivalent
Compile: vil compile --from python --input 010-basic-websocket-chat.py --release
"""
import os
from vil import VilPipeline, VilServer, ServiceProcess

server = VilServer("websocket-chat", port=8080)
chat = server.service_process("chat")
chat.endpoint("GET", "/", "index")
chat.endpoint("GET", "/ws", "ws_handler")
chat.endpoint("GET", "/stats", "stats")
server.compile()
