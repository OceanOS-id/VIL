// 302-rag-multi-source-fanin — Zig SDK equivalent
// Compile: vil compile --from zig --input 302-rag-multi-source-fanin/main.zig --release

const vil = @import("vil");

pub fn main() void {
    var server = vil.Server.init("rag-multi-source-fanin", 3111);
    server.compile();
}
