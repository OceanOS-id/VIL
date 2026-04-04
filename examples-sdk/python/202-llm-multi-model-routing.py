#!/usr/bin/env python3
"""202-llm-multi-model-routing — Python SDK equivalent
Compile: vil compile --from python --input 202-llm-multi-model-routing.py --release
"""
import os
from vil import VilPipeline, VilServer, ServiceProcess

pipeline = VilPipeline("MultiModelPipeline_GPT4", 8080)
pipeline.route("sink.trigger_out", "source_gpt4.trigger_in", "LoanWrite")
pipeline.route("source_gpt4.response_data_out", "sink.response_data_in", "LoanWrite")
pipeline.route("source_gpt4.response_ctrl_out", "sink.response_ctrl_in", "Copy")
pipeline.compile()
