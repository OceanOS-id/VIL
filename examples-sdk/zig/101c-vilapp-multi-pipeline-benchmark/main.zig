// 101c-vilapp-multi-pipeline-benchmark — Zig SDK equivalent
// Compile: vil compile --from zig --input 101c-vilapp-multi-pipeline-benchmark/main.zig --release

const vil = @import("vil");

pub fn main() void {
    var server = vil.Server.init("multi-pipeline-bench", 3090);
    var pipeline = vil.Service.init("pipeline");
    server.service(&pipeline);
    server.compile();
}
