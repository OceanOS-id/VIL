#!/usr/bin/env python3
"""403-agent-code-file-reviewer — Python SDK equivalent
Compile: vil compile --from python --input 403-agent-code-file-reviewer.py --release
"""
import os
from vil import VilPipeline, VilServer, ServiceProcess

server = VilServer("code-file-reviewer-agent", port=3122)
code_review_agent = server.service_process("code-review-agent")
code_review_agent.endpoint("POST", "/code-review", "code_review_handler")
server.compile()
