#!/usr/bin/env python3
"""105-pipeline-multi-workflow — Python SDK equivalent
Compile: vil compile --from python --input 105-pipeline-multi-workflow.py --release
"""
import os
from vil import VilPipeline, VilServer, ServiceProcess

pipeline = VilPipeline("AiGatewayWorkflow", 3097)
pipeline.sink(port=3097, path="/ai", name="ai_gateway_sink")
pipeline.source(url="http://127.0.0.1:4545/v1/chat/completions", format="sse", json_tap="choices[0].delta.content", dialect="openai", name="ai_sse_source")
pipeline.sink(port=3098, path="/credit", name="credit_sink")
pipeline.source(url="http://localhost:18081/api/v1/credits/ndjson?count=100", format="json", name="credit_ndjson_source")
pipeline.sink(port=3099, path="/inventory", name="inventory_sink")
pipeline.source(url="http://localhost:18092/api/v1/products", name="inventory_rest_source")
pipeline.route("ai_sink.trigger_out", "ai_source.trigger_in", "LoanWrite")
pipeline.route("ai_source.response_data_out", "ai_sink.response_data_in", "LoanWrite")
pipeline.route("ai_source.response_ctrl_out", "ai_sink.response_ctrl_in", "Copy")
pipeline.route("credit_sink.trigger_out", "credit_source.trigger_in", "LoanWrite")
pipeline.route("credit_source.response_data_out", "credit_sink.response_data_in", "LoanWrite")
pipeline.route("credit_source.response_ctrl_out", "credit_sink.response_ctrl_in", "Copy")
pipeline.route("inventory_sink.trigger_out", "inventory_source.trigger_in", "LoanWrite")
pipeline.route("inventory_source.response_data_out", "inventory_sink.response_data_in", "LoanWrite")
pipeline.route("inventory_source.response_ctrl_out", "inventory_sink.response_ctrl_in", "Copy")
pipeline.compile()
