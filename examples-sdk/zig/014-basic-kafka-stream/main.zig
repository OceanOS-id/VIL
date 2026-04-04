// 014-basic-kafka-stream — Zig SDK equivalent
// Compile: vil compile --from zig --input 014-basic-kafka-stream/main.zig --release

const vil = @import("vil");

pub fn main() void {
    var server = vil.Server.init("kafka-stream", 8080);
    var kafka = vil.Service.init("kafka");
    kafka.endpoint("GET", "/kafka/config", "kafka_config");
    kafka.endpoint("POST", "/kafka/produce", "kafka_produce");
    kafka.endpoint("GET", "/kafka/consumer", "consumer_info");
    kafka.endpoint("GET", "/kafka/bridge", "bridge_status");
    server.service(&kafka);
    var root = vil.Service.init("root");
    server.service(&root);
    server.compile();
}
