#!/usr/bin/env python3
"""304-rag-citation-extraction — Python SDK equivalent
Compile: vil compile --from python --input 304-rag-citation-extraction.py --release
"""
import os
from vil import VilPipeline, VilServer, ServiceProcess

server = VilServer("rag-citation-extraction", port=3113)
rag_citation = server.service_process("rag-citation")
rag_citation.endpoint("POST", "/cited-rag", "cited_rag_handler")
server.compile()
