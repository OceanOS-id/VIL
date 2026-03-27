#!/usr/bin/env python3
"""010 — WebSocket Chat
Equivalent to: examples/010-basic-websocket-chat (Rust)
Compile: vil compile --from python --input 010-basic-websocket-chat.py --release
"""
import os
from vil import VilServer, ServiceProcess

server = VilServer("websocket-chat", port=8080)

# -- WebSocket events ---------------------------------------------------------
server.ws_event("chat_message", topic="chat.message", fields={
    "from": "String",
    "message": "String",
    "timestamp": "String",
})
server.ws_event("user_joined", topic="chat.presence", fields={
    "username": "String",
})
server.ws_event("user_left", topic="chat.presence", fields={
    "username": "String",
})

# -- ServiceProcess: chat (prefix: /api/chat) ---------------------------------
chat = ServiceProcess("chat")
chat.endpoint("GET", "/", "index")
chat.endpoint("GET", "/ws", "ws_handler", protocol="websocket")
chat.endpoint("GET", "/stats", "stats")
server.service(chat, prefix="/api/chat")

# -- Emit / compile -----------------------------------------------------------
if os.environ.get("VIL_COMPILE_MODE") == "manifest":
    print(server.to_yaml())
else:
    server.compile()
