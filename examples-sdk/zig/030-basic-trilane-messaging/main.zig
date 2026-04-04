// 030-basic-trilane-messaging — Zig SDK equivalent
// Compile: vil compile --from zig --input 030-basic-trilane-messaging/main.zig --release

const vil = @import("vil");

pub fn main() void {
    var server = vil.Server.init("ecommerce-order-pipeline", 8080);
    var gateway = vil.Service.init("gateway");
    server.service(&gateway);
    var fulfillment = vil.Service.init("fulfillment");
    fulfillment.endpoint("GET", "/status", "fulfillment_status");
    server.service(&fulfillment);
    server.compile();
}
