// 702-mq-sqs-send-receive — C# SDK equivalent
// Compile: vil compile --from csharp --input 702-mq-sqs-send-receive/Main.cs --release

#r "sdk/csharp/Vil.cs"

var server = new VilServer("app", 8080);
server.Compile();
