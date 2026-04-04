// 302-rag-multi-source-fanin — Zig SDK equivalent
const vil = @import("vil");
pub fn main() void {
    var p = vil.Pipeline.init("rag-multi-source-fanin", 3111);
    p.compile();
}
