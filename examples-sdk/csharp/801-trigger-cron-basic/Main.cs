// 801-trigger-cron-basic — C# SDK equivalent
// Compile: vil compile --from csharp --input 801-trigger-cron-basic/Main.cs --release

#r "sdk/csharp/Vil.cs"

var server = new VilServer("app", 8080);
server.Compile();
