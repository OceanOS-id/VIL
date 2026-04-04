#!/usr/bin/env python3
"""101b-multi-pipeline-benchmark — Python SDK equivalent
Compile: vil compile --from python --input 101b-multi-pipeline-benchmark.py --release
"""
import os
from vil import VilPipeline, VilServer, ServiceProcess

pipeline = VilPipeline("MultiPipelineBench", 3090)
pipeline.sink(port=3090, path="/trigger", name="gateway")
pipeline.source(url="http://127.0.0.1:4545/v1/chat/completions", json_tap="choices[0].delta.content", name="l_l_m_upstream")
pipeline.route("sink.trigger_out", "source.trigger_in", "LoanWrite")
pipeline.route("source.response_data_out", "sink.response_data_in", "LoanWrite")
pipeline.route("source.response_ctrl_out", "sink.response_ctrl_in", "Copy")
pipeline.compile()
