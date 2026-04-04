// 101-pipeline-3node-transform-chain — Zig SDK equivalent
// Compile: vil compile --from zig --input 101-pipeline-3node-transform-chain/main.zig --release

const vil = @import("vil");

pub fn main() void {
    var p = vil.Pipeline.init("TransformChainPipeline", 3090);
    p.sink("transform_gateway", 3090, "/transform");
    p.source("chained_transform_source", "http://localhost:18081/api/v1/credits/ndjson?count=100", "json");
    p.route("sink.trigger_out", "source.trigger_in", "LoanWrite");
    p.route("source.response_data_out", "sink.response_data_in", "LoanWrite");
    p.route("source.response_ctrl_out", "sink.response_ctrl_in", "Copy");
    p.compile();
}
