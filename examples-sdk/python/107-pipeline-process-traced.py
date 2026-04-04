#!/usr/bin/env python3
"""107-pipeline-process-traced — Python SDK equivalent
Compile: vil compile --from python --input 107-pipeline-process-traced.py --release
"""
import os
from vil import VilPipeline, VilServer, ServiceProcess

pipeline = VilPipeline("SupplyChainTrackedPipeline", 3107)
pipeline.sink(port=3107, path="/traced", name="tracking_sink")
pipeline.source(url="http://localhost:18081/api/v1/credits/stream", format="sse", name="supply_chain_source")
pipeline.route("sink.trigger_out", "source.trigger_in", "LoanWrite")
pipeline.route("source.tracking_data_out", "sink.tracking_data_in", "LoanWrite")
pipeline.route("source.delivery_ctrl_out", "sink.delivery_ctrl_in", "Copy")
pipeline.compile()
