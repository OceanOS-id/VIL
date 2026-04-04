// 017-basic-production-fullstack — Zig SDK equivalent
// Compile: vil compile --from zig --input 017-basic-production-fullstack/main.zig --release

const vil = @import("vil");

pub fn main() void {
    var server = vil.Server.init("production-fullstack", 8080);
    var fullstack = vil.Service.init("fullstack");
    fullstack.endpoint("GET", "/stack", "stack_info");
    fullstack.endpoint("GET", "/config", "full_config");
    fullstack.endpoint("GET", "/sprints", "sprints");
    fullstack.endpoint("GET", "/middleware", "middleware_info");
    server.service(&fullstack);
    var admin = vil.Service.init("admin");
    admin.endpoint("GET", "/config", "full_config");
    server.service(&admin);
    var root = vil.Service.init("root");
    server.service(&root);
    server.compile();
}
