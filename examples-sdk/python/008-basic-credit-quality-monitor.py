#!/usr/bin/env python3
"""008-basic-credit-quality-monitor — Python SDK equivalent
Compile: vil compile --from python --input 008-basic-credit-quality-monitor.py --release
"""
import os
from vil import VilPipeline, VilServer, ServiceProcess

pipeline = VilPipeline("CreditQualityMonitorPipeline", 3082)
pipeline.sink(port=3082, path="/quality-check", name="quality_monitor_sink")
pipeline.source(format="json", name="quality_credit_source")
pipeline.route("sink.trigger_out", "source.trigger_in", "LoanWrite")
pipeline.route("source.response_data_out", "sink.response_data_in", "LoanWrite")
pipeline.route("source.response_ctrl_out", "sink.response_ctrl_in", "Copy")
pipeline.compile()
