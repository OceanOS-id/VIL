#!/usr/bin/env python3
"""102-pipeline-fanout-scatter — Python SDK equivalent
Compile: vil compile --from python --input 102-pipeline-fanout-scatter.py --release
"""
import os
from vil import VilPipeline, VilServer, ServiceProcess

pipeline = VilPipeline("NplPipeline", 3091)
pipeline.sink(port=3091, path="/npl", name="npl_sink")
pipeline.source(url="http://localhost:18081/api/v1/credits/ndjson?count=100", format="json", name="npl_source")
pipeline.sink(port=3092, path="/healthy", name="healthy_sink")
pipeline.source(url="http://localhost:18081/api/v1/credits/ndjson?count=100", format="json", name="healthy_source")
pipeline.route("npl_sink.trigger_out", "npl_source.trigger_in", "LoanWrite")
pipeline.route("npl_source.response_data_out", "npl_sink.response_data_in", "LoanWrite")
pipeline.route("npl_source.response_ctrl_out", "npl_sink.response_ctrl_in", "Copy")
pipeline.route("healthy_sink.trigger_out", "healthy_source.trigger_in", "LoanWrite")
pipeline.route("healthy_source.response_data_out", "healthy_sink.response_data_in", "LoanWrite")
pipeline.route("healthy_source.response_ctrl_out", "healthy_sink.response_ctrl_in", "Copy")
pipeline.compile()
