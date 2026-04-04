#!/usr/bin/env python3
"""702-mq-sqs-send-receive — Python SDK equivalent
Compile: vil compile --from python --input 702-mq-sqs-send-receive.py --release
"""
import os
from vil import VilPipeline, VilServer, ServiceProcess

server = VilServer("app", port=8080)
server.compile()
