#!/usr/bin/env python3
"""801-trigger-cron-basic — Python SDK equivalent
Compile: vil compile --from python --input 801-trigger-cron-basic.py --release
"""
import os
from vil import VilPipeline, VilServer, ServiceProcess

server = VilServer("app", port=8080)
server.compile()
