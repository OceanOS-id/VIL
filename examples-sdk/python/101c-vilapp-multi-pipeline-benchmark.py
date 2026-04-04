#!/usr/bin/env python3
"""101c-vilapp-multi-pipeline-benchmark — Python SDK equivalent
Compile: vil compile --from python --input 101c-vilapp-multi-pipeline-benchmark.py --release
"""
import os
from vil import VilPipeline, VilServer, ServiceProcess

server = VilServer("multi-pipeline-bench", port=3090)
pipeline = server.service_process("pipeline")
server.compile()
