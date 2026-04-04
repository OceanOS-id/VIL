// 504-villog-benchmark-comparison — C# SDK equivalent
// Compile: vil compile --from csharp --input 504-villog-benchmark-comparison/Main.cs --release

#r "sdk/csharp/Vil.cs"

var server = new VilServer("app", 8080);
server.Compile();
