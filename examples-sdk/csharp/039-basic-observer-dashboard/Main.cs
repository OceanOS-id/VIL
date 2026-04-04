// 039-basic-observer-dashboard — C# SDK equivalent
// Compile: vil compile --from csharp --input 039-basic-observer-dashboard/Main.cs --release

#r "sdk/csharp/Vil.cs"

var server = new VilServer("observer-demo", 8080);
var demo = new ServiceProcess("demo");
demo.Endpoint("GET", "/hello", "hello");
demo.Endpoint("POST", "/echo", "echo");
server.Service(demo);
server.Compile();
