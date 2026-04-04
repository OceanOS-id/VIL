// 018-basic-ai-multi-model-router — C# SDK equivalent
// Compile: vil compile --from csharp --input 018-basic-ai-multi-model-router/Main.cs --release

#r "sdk/csharp/Vil.cs"

var server = new VilServer("ai-multi-model-router", 3085);
var router = new ServiceProcess("router");
router.Endpoint("POST", "/route", "route_handler");
server.Service(router);
server.Compile();
