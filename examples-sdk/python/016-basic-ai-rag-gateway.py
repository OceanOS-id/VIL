#!/usr/bin/env python3
"""016-basic-ai-rag-gateway — Python SDK equivalent
Compile: vil compile --from python --input 016-basic-ai-rag-gateway.py --release
"""
import os
from vil import VilPipeline, VilServer, ServiceProcess

pipeline = VilPipeline("RagPipeline", 3084)
pipeline.sink(port=3084, path="/rag", name="rag_webhook")
pipeline.source(url="http://127.0.0.1:4545/v1/chat/completions", format="sse", json_tap="choices[0].delta.content", dialect="openai", name="rag_sse_inference")
pipeline.route("sink.trigger_out", "source.trigger_in", "LoanWrite")
pipeline.route("source.response_data_out", "sink.response_data_in", "LoanWrite")
pipeline.route("source.response_ctrl_out", "sink.response_ctrl_in", "Copy")
pipeline.compile()
