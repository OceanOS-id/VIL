#!/usr/bin/env python3
"""104-pipeline-diamond-topology — Python SDK equivalent
Compile: vil compile --from python --input 104-pipeline-diamond-topology.py --release
"""
import os
from vil import VilPipeline, VilServer, ServiceProcess

pipeline = VilPipeline("DiamondSummary", 3095)
pipeline.sink(port=3095, path="/diamond", name="summary_sink")
pipeline.source(url="http://localhost:18081/api/v1/credits/ndjson?count=100", format="json", name="summary_source")
pipeline.sink(port=3096, path="/diamond-detail", name="detail_sink")
pipeline.source(url="http://localhost:18081/api/v1/credits/ndjson?count=100", format="json", name="detail_source")
pipeline.route("summary_sink.trigger_out", "summary_source.trigger_in", "LoanWrite")
pipeline.route("summary_source.response_data_out", "summary_sink.response_data_in", "LoanWrite")
pipeline.route("summary_source.response_ctrl_out", "summary_sink.response_ctrl_in", "Copy")
pipeline.route("detail_sink.trigger_out", "detail_source.trigger_in", "LoanWrite")
pipeline.route("detail_source.response_data_out", "detail_sink.response_data_in", "LoanWrite")
pipeline.route("detail_source.response_ctrl_out", "detail_sink.response_ctrl_in", "Copy")
pipeline.compile()
