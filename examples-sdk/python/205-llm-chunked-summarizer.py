#!/usr/bin/env python3
"""205-llm-chunked-summarizer — Python SDK equivalent
Compile: vil compile --from python --input 205-llm-chunked-summarizer.py --release
"""
import os
from vil import VilPipeline, VilServer, ServiceProcess

server = VilServer("ChunkedSummarizerPipeline", port=8080)
server.compile()
