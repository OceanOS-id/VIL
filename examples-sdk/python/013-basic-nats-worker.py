#!/usr/bin/env python3
"""013-basic-nats-worker — Python SDK equivalent
Compile: vil compile --from python --input 013-basic-nats-worker.py --release
"""
import os
from vil import VilPipeline, VilServer, ServiceProcess

server = VilServer("nats-worker", port=8080)
nats = server.service_process("nats")
nats.endpoint("GET", "/nats/config", "nats_config")
nats.endpoint("POST", "/nats/publish", "nats_publish")
nats.endpoint("GET", "/nats/jetstream", "jetstream_info")
nats.endpoint("GET", "/nats/kv", "kv_demo")
root = server.service_process("root")
server.compile()
