#!/usr/bin/env python3
"""301-rag-basic-vector-search — Python SDK equivalent
Compile: vil compile --from python --input 301-rag-basic-vector-search.py --release
"""
import os
from vil import VilPipeline, VilServer, ServiceProcess

server = VilServer("rag-basic-vector-search", port=3110)
rag_basic = server.service_process("rag-basic")
rag_basic.endpoint("POST", "/rag", "rag_handler")
server.compile()
