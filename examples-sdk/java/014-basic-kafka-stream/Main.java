// 014-basic-kafka-stream — Java SDK equivalent
// Compile: vil compile --from java --input 014-basic-kafka-stream/Main.java --release

import dev.vil.*;

public class Main {
    public static void main(String[] args) {
        VilServer server = new VilServer("kafka-stream", 8080);
        ServiceProcess kafka = new ServiceProcess("kafka");
        kafka.endpoint("GET", "/kafka/config", "kafka_config");
        kafka.endpoint("POST", "/kafka/produce", "kafka_produce");
        kafka.endpoint("GET", "/kafka/consumer", "consumer_info");
        kafka.endpoint("GET", "/kafka/bridge", "bridge_status");
        server.service(kafka);
        ServiceProcess root = new ServiceProcess("root");
        server.service(root);
        server.compile();
    }
}
