#!/usr/bin/env python3
"""502-villog-file-rolling — Python SDK equivalent
Compile: vil compile --from python --input 502-villog-file-rolling.py --release
"""
import os
from vil import VilPipeline, VilServer, ServiceProcess

server = VilServer("app", port=8080)
server.compile()
