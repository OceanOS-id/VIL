// 503-villog-multi-drain — C# SDK equivalent
// Compile: vil compile --from csharp --input 503-villog-multi-drain/Main.cs --release

#r "sdk/csharp/Vil.cs"

var server = new VilServer("app", 8080);
server.Compile();
