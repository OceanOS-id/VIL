// 603-db-clickhouse-batch — C# SDK equivalent
// Compile: vil compile --from csharp --input 603-db-clickhouse-batch/Main.cs --release

#r "sdk/csharp/Vil.cs"

var server = new VilServer("app", 8080);
server.Compile();
