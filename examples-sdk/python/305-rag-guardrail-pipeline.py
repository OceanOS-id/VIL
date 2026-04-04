#!/usr/bin/env python3
"""305-rag-guardrail-pipeline — Python SDK equivalent
Compile: vil compile --from python --input 305-rag-guardrail-pipeline.py --release
"""
import os
from vil import VilPipeline, VilServer, ServiceProcess

server = VilServer("rag-guardrail-pipeline", port=3114)
rag_guardrail = server.service_process("rag-guardrail")
rag_guardrail.endpoint("POST", "/safe-rag", "safe_rag_handler")
server.compile()
