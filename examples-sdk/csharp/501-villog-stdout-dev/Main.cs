// 501-villog-stdout-dev — C# SDK equivalent
// Compile: vil compile --from csharp --input 501-villog-stdout-dev/Main.cs --release

#r "sdk/csharp/Vil.cs"

var server = new VilServer("app", 8080);
server.Compile();
