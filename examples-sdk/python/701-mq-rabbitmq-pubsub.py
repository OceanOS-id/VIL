#!/usr/bin/env python3
"""701-mq-rabbitmq-pubsub — Python SDK equivalent
Compile: vil compile --from python --input 701-mq-rabbitmq-pubsub.py --release
"""
import os
from vil import VilPipeline, VilServer, ServiceProcess

server = VilServer("app", port=8080)
server.compile()
