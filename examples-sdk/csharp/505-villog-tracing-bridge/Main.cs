// 505-villog-tracing-bridge — C# SDK equivalent
// Compile: vil compile --from csharp --input 505-villog-tracing-bridge/Main.cs --release

#r "sdk/csharp/Vil.cs"

var server = new VilServer("app", 8080);
server.Compile();
