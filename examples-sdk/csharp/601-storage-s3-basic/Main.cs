// 601-storage-s3-basic — C# SDK equivalent
// Compile: vil compile --from csharp --input 601-storage-s3-basic/Main.cs --release

#r "sdk/csharp/Vil.cs"

var server = new VilServer("app", 8080);
server.Compile();
