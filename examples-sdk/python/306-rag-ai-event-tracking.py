#!/usr/bin/env python3
"""306-rag-ai-event-tracking — Python SDK equivalent
Compile: vil compile --from python --input 306-rag-ai-event-tracking.py --release
"""
import os
from vil import VilPipeline, VilServer, ServiceProcess

server = VilServer("customer-support-rag", port=3116)
support = server.service_process("support")
support.endpoint("POST", "/support/ask", "answer_question")
server.compile()
