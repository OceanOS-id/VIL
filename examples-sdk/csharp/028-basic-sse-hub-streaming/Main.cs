// 028-basic-sse-hub-streaming — C# SDK equivalent
// Compile: vil compile --from csharp --input 028-basic-sse-hub-streaming/Main.cs --release

#r "sdk/csharp/Vil.cs"

var server = new VilServer("sse-hub-demo", 8080);
var events = new ServiceProcess("events");
events.Endpoint("POST", "/publish", "publish");
events.Endpoint("GET", "/stream", "stream");
events.Endpoint("GET", "/stats", "stats");
server.Service(events);
server.Compile();
