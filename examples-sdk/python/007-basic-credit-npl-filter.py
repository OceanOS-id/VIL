#!/usr/bin/env python3
"""007-basic-credit-npl-filter — Python SDK equivalent
Compile: vil compile --from python --input 007-basic-credit-npl-filter.py --release
"""
import os
from vil import VilPipeline, VilServer, ServiceProcess

pipeline = VilPipeline("NplFilterPipeline", 3081)
pipeline.sink(port=3081, path="/filter-npl", name="npl_filter_sink")
pipeline.source(format="json", name="npl_credit_source")
pipeline.route("sink.trigger_out", "source.trigger_in", "LoanWrite")
pipeline.route("source.response_data_out", "sink.response_data_in", "LoanWrite")
pipeline.route("source.response_ctrl_out", "sink.response_ctrl_in", "Copy")
pipeline.compile()
