// 027-basic-vilserver-minimal — Zig SDK equivalent
// Compile: vil compile --from zig --input 027-basic-vilserver-minimal/main.zig --release

const vil = @import("vil");

pub fn main() void {
    var server = vil.Server.init("app", 8080);
    server.compile();
}
