// 509-villog-phase1-integration — C# SDK equivalent
// Compile: vil compile --from csharp --input 509-villog-phase1-integration/Main.cs --release

#r "sdk/csharp/Vil.cs"

var server = new VilServer("app", 8080);
server.Compile();
