// 802-trigger-fs-watcher — C# SDK equivalent
// Compile: vil compile --from csharp --input 802-trigger-fs-watcher/Main.cs --release

#r "sdk/csharp/Vil.cs"

var server = new VilServer("app", 8080);
server.Compile();
