// 601-storage-s3-basic — Zig SDK equivalent
// Compile: vil compile --from zig --input 601-storage-s3-basic/main.zig --release

const vil = @import("vil");

pub fn main() void {
    var server = vil.Server.init("app", 8080);
    server.compile();
}
