#!/usr/bin/env python3
"""009-basic-credit-regulatory-slik — Python SDK equivalent
Compile: vil compile --from python --input 009-basic-credit-regulatory-slik.py --release
"""
import os
from vil import VilPipeline, VilServer, ServiceProcess

pipeline = VilPipeline("RegulatoryStreamPipeline", 3083)
pipeline.sink(port=3083, path="/regulatory-stream", name="regulatory_sink")
pipeline.source(url="http://localhost:18081/api/v1/credits/ndjson?count=1000", format="json", name="regulatory_source")
pipeline.route("sink.trigger_out", "source.trigger_in", "LoanWrite")
pipeline.route("source.response_data_out", "sink.response_data_in", "LoanWrite")
pipeline.route("source.response_ctrl_out", "sink.response_ctrl_in", "Copy")
pipeline.compile()
