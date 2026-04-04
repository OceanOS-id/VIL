// 037-basic-vilmodel-derive — Zig SDK equivalent
// Compile: vil compile --from zig --input 037-basic-vilmodel-derive/main.zig --release

const vil = @import("vil");

pub fn main() void {
    var server = vil.Server.init("insurance-claim-processing", 8080);
    var claims = vil.Service.init("claims");
    claims.endpoint("POST", "/claims/submit", "submit_claim");
    claims.endpoint("GET", "/claims/sample", "sample_claim");
    server.service(&claims);
    server.compile();
}
