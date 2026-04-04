#!/usr/bin/env python3
"""203-llm-code-review-with-tools — Python SDK equivalent
Compile: vil compile --from python --input 203-llm-code-review-with-tools.py --release
"""
import os
from vil import VilPipeline, VilServer, ServiceProcess

server = VilServer("llm-code-review-tools", port=3102)
code_review = server.service_process("code-review")
code_review.endpoint("POST", "/code/review", "code_review_handler")
server.compile()
