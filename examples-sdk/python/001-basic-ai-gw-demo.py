#!/usr/bin/env python3
"""001-basic-ai-gw-demo — Python SDK equivalent
Compile: vil compile --from python --input 001-basic-ai-gw-demo.py --release
"""
import os
from vil import VilPipeline, VilServer, ServiceProcess

pipeline = VilPipeline("DecomposedPipeline", 3080)
pipeline.sink(port=3080, path="/trigger", name="webhook_trigger")
pipeline.source(url="http://127.0.0.1:4545/v1/chat/completions", format="sse", json_tap="choices[0].delta.content", dialect="openai", name="sse_inference")
pipeline.route("sink.trigger_out", "source.trigger_in", "LoanWrite")
pipeline.route("source.response_data_out", "sink.response_data_in", "LoanWrite")
pipeline.route("source.response_ctrl_out", "sink.response_ctrl_in", "Copy")
pipeline.compile()
