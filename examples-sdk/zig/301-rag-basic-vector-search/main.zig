// 301-rag-basic-vector-search — Zig SDK equivalent
// Compile: vil compile --from zig --input 301-rag-basic-vector-search/main.zig --release

const vil = @import("vil");

pub fn main() void {
    var server = vil.Server.init("rag-basic-vector-search", 3110);
    var rag_basic = vil.Service.init("rag-basic");
    rag_basic.endpoint("POST", "/rag", "rag_handler");
    server.service(&rag_basic);
    server.compile();
}
