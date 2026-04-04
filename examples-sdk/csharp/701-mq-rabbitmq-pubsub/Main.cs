// 701-mq-rabbitmq-pubsub — C# SDK equivalent
// Compile: vil compile --from csharp --input 701-mq-rabbitmq-pubsub/Main.cs --release

#r "sdk/csharp/Vil.cs"

var server = new VilServer("app", 8080);
server.Compile();
