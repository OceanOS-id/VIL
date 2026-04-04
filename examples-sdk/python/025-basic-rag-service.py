#!/usr/bin/env python3
"""025-basic-rag-service — Python SDK equivalent
Compile: vil compile --from python --input 025-basic-rag-service.py --release
"""
import os
from vil import VilPipeline, VilServer, ServiceProcess

server = VilServer("rag-service", port=3091)
rag = server.service_process("rag")
rag.endpoint("POST", "/rag", "rag_handler")
server.compile()
