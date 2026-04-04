#!/usr/bin/env python3
"""103-pipeline-fanin-gather — Python SDK equivalent
Compile: vil compile --from python --input 103-pipeline-fanin-gather.py --release
"""
import os
from vil import VilPipeline, VilServer, ServiceProcess

pipeline = VilPipeline("CreditGatherPipeline", 3093)
pipeline.sink(port=3093, path="/gather", name="credit_gather_sink")
pipeline.source(url="http://localhost:18081/api/v1/credits/ndjson?count=100", format="json", name="credit_source")
pipeline.sink(port=3094, path="/inventory", name="inventory_gather_sink")
pipeline.source(url="http://localhost:18092/api/v1/products", name="inventory_source")
pipeline.route("credit_sink.trigger_out", "credit_source.trigger_in", "LoanWrite")
pipeline.route("credit_source.response_data_out", "credit_sink.response_data_in", "LoanWrite")
pipeline.route("credit_source.response_ctrl_out", "credit_sink.response_ctrl_in", "Copy")
pipeline.route("inventory_sink.trigger_out", "inventory_source.trigger_in", "LoanWrite")
pipeline.route("inventory_source.response_data_out", "inventory_sink.response_data_in", "LoanWrite")
pipeline.route("inventory_source.response_ctrl_out", "inventory_sink.response_ctrl_in", "Copy")
pipeline.compile()
