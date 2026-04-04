// 025-basic-rag-service — Zig SDK equivalent
// Compile: vil compile --from zig --input 025-basic-rag-service/main.zig --release

const vil = @import("vil");

pub fn main() void {
    var server = vil.Server.init("rag-service", 3091);
    var rag = vil.Service.init("rag");
    rag.endpoint("POST", "/rag", "rag_handler");
    server.service(&rag);
    server.compile();
}
