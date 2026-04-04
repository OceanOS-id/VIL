// 030-basic-trilane-messaging — C# SDK equivalent
// Compile: vil compile --from csharp --input 030-basic-trilane-messaging/Main.cs --release

#r "sdk/csharp/Vil.cs"

var server = new VilServer("ecommerce-order-pipeline", 8080);
var gateway = new ServiceProcess("gateway");
server.Service(gateway);
var fulfillment = new ServiceProcess("fulfillment");
fulfillment.Endpoint("GET", "/status", "fulfillment_status");
server.Service(fulfillment);
server.Compile();
