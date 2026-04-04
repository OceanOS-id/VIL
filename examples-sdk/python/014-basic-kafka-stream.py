#!/usr/bin/env python3
"""014-basic-kafka-stream — Python SDK equivalent
Compile: vil compile --from python --input 014-basic-kafka-stream.py --release
"""
import os
from vil import VilPipeline, VilServer, ServiceProcess

server = VilServer("kafka-stream", port=8080)
kafka = server.service_process("kafka")
kafka.endpoint("GET", "/kafka/config", "kafka_config")
kafka.endpoint("POST", "/kafka/produce", "kafka_produce")
kafka.endpoint("GET", "/kafka/consumer", "consumer_info")
kafka.endpoint("GET", "/kafka/bridge", "bridge_status")
root = server.service_process("root")
server.compile()
