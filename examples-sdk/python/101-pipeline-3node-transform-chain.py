#!/usr/bin/env python3
"""101-pipeline-3node-transform-chain — Python SDK equivalent
Compile: vil compile --from python --input 101-pipeline-3node-transform-chain.py --release
"""
import os
from vil import VilPipeline, VilServer, ServiceProcess

pipeline = VilPipeline("TransformChainPipeline", 3090)
pipeline.sink(port=3090, path="/transform", name="transform_gateway")
pipeline.source(url="http://localhost:18081/api/v1/credits/ndjson?count=100", format="json", name="chained_transform_source")
pipeline.route("sink.trigger_out", "source.trigger_in", "LoanWrite")
pipeline.route("source.response_data_out", "sink.response_data_in", "LoanWrite")
pipeline.route("source.response_ctrl_out", "sink.response_ctrl_in", "Copy")
pipeline.compile()
