#!/usr/bin/env python3
"""303-rag-hybrid-exact-semantic — Python SDK equivalent
Compile: vil compile --from python --input 303-rag-hybrid-exact-semantic.py --release
"""
import os
from vil import VilPipeline, VilServer, ServiceProcess

server = VilServer("rag-hybrid-exact-semantic", port=3112)
rag_hybrid = server.service_process("rag-hybrid")
rag_hybrid.endpoint("POST", "/hybrid", "hybrid_handler")
server.compile()
