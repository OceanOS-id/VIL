#!/usr/bin/env python3
"""205-llm-chunked-summarizer — Python SDK equivalent
Compile: vil compile --from python --input 205-llm-chunked-summarizer.py --release
"""
import os
from vil import VilPipeline, VilServer, ServiceProcess

pipeline = VilPipeline("ChunkedSummarizerPipeline", 8080)
pipeline.route("sink.trigger_out", "source_summarize.trigger_in", "LoanWrite")
pipeline.route("source_summarize.response_data_out", "sink.response_data_in", "LoanWrite")
pipeline.route("source_summarize.response_ctrl_out", "sink.response_ctrl_in", "Copy")
pipeline.compile()
