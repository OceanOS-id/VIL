// 031-basic-mesh-routing — C# SDK equivalent
// Compile: vil compile --from csharp --input 031-basic-mesh-routing/Main.cs --release

#r "sdk/csharp/Vil.cs"

var server = new VilServer("banking-transaction-mesh", 8080);
var teller = new ServiceProcess("teller");
teller.Endpoint("GET", "/ping", "teller_ping");
teller.Endpoint("POST", "/submit", "teller_submit");
server.Service(teller);
var fraud_check = new ServiceProcess("fraud_check");
fraud_check.Endpoint("POST", "/analyze", "fraud_process");
server.Service(fraud_check);
var core_banking = new ServiceProcess("core_banking");
core_banking.Endpoint("POST", "/post", "core_banking_post");
server.Service(core_banking);
var notification = new ServiceProcess("notification");
notification.Endpoint("GET", "/send", "notification_send");
server.Service(notification);
server.Compile();
