// 035-basic-vil-service-module — C# SDK equivalent
// Compile: vil compile --from csharp --input 035-basic-vil-service-module/Main.cs --release

#r "sdk/csharp/Vil.cs"

var server = new VilServer("app", 8080);
server.Compile();
