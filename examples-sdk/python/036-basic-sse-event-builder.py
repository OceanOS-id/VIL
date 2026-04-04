#!/usr/bin/env python3
"""036-basic-sse-event-builder — Python SDK equivalent
Compile: vil compile --from python --input 036-basic-sse-event-builder.py --release
"""
import os
from vil import VilPipeline, VilServer, ServiceProcess

server = VilServer("stock-market-ticker", port=8080)
ticker = server.service_process("ticker")
ticker.endpoint("GET", "/stream", "ticker_stream")
ticker.endpoint("GET", "/info", "ticker_info")
server.compile()
