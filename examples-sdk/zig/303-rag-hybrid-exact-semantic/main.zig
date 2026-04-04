// 303-rag-hybrid-exact-semantic — Zig SDK equivalent
// Compile: vil compile --from zig --input 303-rag-hybrid-exact-semantic/main.zig --release

const vil = @import("vil");

pub fn main() void {
    var server = vil.Server.init("rag-hybrid-exact-semantic", 3112);
    var rag_hybrid = vil.Service.init("rag-hybrid");
    rag_hybrid.endpoint("POST", "/hybrid", "hybrid_handler");
    server.service(&rag_hybrid);
    server.compile();
}
