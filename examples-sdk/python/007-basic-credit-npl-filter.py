#!/usr/bin/env python3
"""007 — Credit NPL Filter (.transform)
Equivalent to: examples/007-basic-credit-npl-filter (Rust)
Compile: vil compile --from python --input 007-basic-credit-npl-filter.py --release
"""
import os
from vil import VilPipeline

pipeline = VilPipeline("credit-npl-filter", port=3081)

# -- Semantic types -----------------------------------------------------------
pipeline.semantic_type("CreditRecord", "event", fields={
    "loan_id": "u64",
    "kolektabilitas": "u8",
    "outstanding": "f64",
})
pipeline.fault("FilterFault", variants=["ParseError", "UpstreamTimeout"])

# -- Nodes --------------------------------------------------------------------
pipeline.source(
    url="http://localhost:18081/api/v1/credits/ndjson?count=1000",
    format="ndjson",
    name="credits",
)
pipeline.sink(port=3081, path="/filter", name="output")

# -- Transform: keep only NPL (kolektabilitas >= 3) ---------------------------
pipeline.transform("npl_filter", """
    let record = serde_json::from_slice(line).ok()?;
    if record["kolektabilitas"].as_u64()? >= 3 { Some(line.to_vec()) } else { None }
""")

# -- Tri-Lane routes ----------------------------------------------------------
pipeline.route("credits.data_out", "npl_filter.data_in", "LoanWrite")
pipeline.route("npl_filter.data_out", "output.data_in", "LoanWrite")
pipeline.route("credits.ctrl_out", "output.ctrl_in", "Copy")

# -- Emit / compile -----------------------------------------------------------
if os.environ.get("VIL_COMPILE_MODE") == "manifest":
    print(pipeline.to_yaml())
else:
    pipeline.compile()
