#!/usr/bin/env python3
"""019-basic-ai-multi-model-advanced — Python SDK equivalent
Compile: vil compile --from python --input 019-basic-ai-multi-model-advanced.py --release
"""
import os
from vil import VilPipeline, VilServer, ServiceProcess

pipeline = VilPipeline("AdvancedMultiModelRouterPipeline", 3086)
pipeline.sink(port=3086, path="/route-advanced", name="advanced_router_sink")
pipeline.source(url="http://127.0.0.1:4545/v1/chat/completions", format="sse", json_tap="choices[0].delta.content", dialect="openai", name="advanced_router_source")
pipeline.route("sink.trigger_out", "source.trigger_in", "LoanWrite")
pipeline.route("source.response_data_out", "sink.response_data_in", "LoanWrite")
pipeline.route("source.response_ctrl_out", "sink.response_ctrl_in", "Copy")
pipeline.compile()
