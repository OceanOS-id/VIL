// 502-villog-file-rolling — C# SDK equivalent
// Compile: vil compile --from csharp --input 502-villog-file-rolling/Main.cs --release

#r "sdk/csharp/Vil.cs"

var server = new VilServer("app", 8080);
server.Compile();
