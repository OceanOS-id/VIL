// 508-villog-bench-multithread — C# SDK equivalent
// Compile: vil compile --from csharp --input 508-villog-bench-multithread/Main.cs --release

#r "sdk/csharp/Vil.cs"

var server = new VilServer("app", 8080);
server.Compile();
