// 014-basic-kafka-stream — C# SDK equivalent
// Compile: vil compile --from csharp --input 014-basic-kafka-stream/Main.cs --release

#r "sdk/csharp/Vil.cs"

var server = new VilServer("kafka-stream", 8080);
var kafka = new ServiceProcess("kafka");
kafka.Endpoint("GET", "/kafka/config", "kafka_config");
kafka.Endpoint("POST", "/kafka/produce", "kafka_produce");
kafka.Endpoint("GET", "/kafka/consumer", "consumer_info");
kafka.Endpoint("GET", "/kafka/bridge", "bridge_status");
server.Service(kafka);
var root = new ServiceProcess("root");
server.Service(root);
server.Compile();
