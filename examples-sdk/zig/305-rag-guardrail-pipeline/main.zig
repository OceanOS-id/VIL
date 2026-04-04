// 305-rag-guardrail-pipeline — Zig SDK equivalent
// Compile: vil compile --from zig --input 305-rag-guardrail-pipeline/main.zig --release

const vil = @import("vil");

pub fn main() void {
    var server = vil.Server.init("rag-guardrail-pipeline", 3114);
    var rag_guardrail = vil.Service.init("rag-guardrail");
    rag_guardrail.endpoint("POST", "/safe-rag", "safe_rag_handler");
    server.service(&rag_guardrail);
    server.compile();
}
