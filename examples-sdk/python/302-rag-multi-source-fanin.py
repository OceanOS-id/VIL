#!/usr/bin/env python3
"""302-rag-multi-source-fanin — Python SDK equivalent
Compile: vil compile --from python --input 302-rag-multi-source-fanin.py --release
"""
import os
from vil import VilPipeline, VilServer, ServiceProcess

server = VilServer("rag-multi-source-fanin", port=3111)
server.compile()
