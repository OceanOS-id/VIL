// 507-villog-bench-file-drain — C# SDK equivalent
// Compile: vil compile --from csharp --input 507-villog-bench-file-drain/Main.cs --release

#r "sdk/csharp/Vil.cs"

var server = new VilServer("app", 8080);
server.Compile();
