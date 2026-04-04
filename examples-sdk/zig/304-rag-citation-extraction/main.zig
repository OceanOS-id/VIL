// 304-rag-citation-extraction — Zig SDK equivalent
// Compile: vil compile --from zig --input 304-rag-citation-extraction/main.zig --release

const vil = @import("vil");

pub fn main() void {
    var server = vil.Server.init("rag-citation-extraction", 3113);
    var rag_citation = vil.Service.init("rag-citation");
    rag_citation.endpoint("POST", "/cited-rag", "cited_rag_handler");
    server.service(&rag_citation);
    server.compile();
}
