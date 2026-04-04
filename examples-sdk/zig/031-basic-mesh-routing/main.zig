// 031-basic-mesh-routing — Zig SDK equivalent
// Compile: vil compile --from zig --input 031-basic-mesh-routing/main.zig --release

const vil = @import("vil");

pub fn main() void {
    var server = vil.Server.init("banking-transaction-mesh", 8080);
    var teller = vil.Service.init("teller");
    teller.endpoint("GET", "/ping", "teller_ping");
    teller.endpoint("POST", "/submit", "teller_submit");
    server.service(&teller);
    var fraud_check = vil.Service.init("fraud_check");
    fraud_check.endpoint("POST", "/analyze", "fraud_process");
    server.service(&fraud_check);
    var core_banking = vil.Service.init("core_banking");
    core_banking.endpoint("POST", "/post", "core_banking_post");
    server.service(&core_banking);
    var notification = vil.Service.init("notification");
    notification.endpoint("GET", "/send", "notification_send");
    server.service(&notification);
    server.compile();
}
