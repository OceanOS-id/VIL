#!/usr/bin/env python3
"""202-llm-multi-model-routing — Python SDK equivalent
Compile: vil compile --from python --input 202-llm-multi-model-routing.py --release
"""
import os
from vil import VilPipeline, VilServer, ServiceProcess

server = VilServer("MultiModelPipeline_GPT4", port=8080)
server.compile()
