// 005-basic-multiservice-mesh-ndjson — Zig SDK equivalent
// Compile: vil compile --from zig --input 005-basic-multiservice-mesh-ndjson/main.zig --release

const vil = @import("vil");

pub fn main() void {
    var p = vil.Pipeline.init("MultiServiceMesh", 3084);
    p.sink("gateway", 3084, "/ingest");
    p.source("credit_ingest", "json");
    p.route("gateway.trigger_out", "ingest.trigger_in", "LoanWrite");
    p.route("ingest.response_data_out", "gateway.response_data_in", "LoanWrite");
    p.route("ingest.response_ctrl_out", "gateway.response_ctrl_in", "Copy");
    p.compile();
}
