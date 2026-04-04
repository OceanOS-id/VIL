// 015-basic-mqtt-iot-gateway — Java SDK equivalent
// Compile: vil compile --from java --input 015-basic-mqtt-iot-gateway/Main.java --release

import dev.vil.*;

public class Main {
    public static void main(String[] args) {
        VilServer server = new VilServer("mqtt-iot-gateway", 8080);
        ServiceProcess mqtt_iot = new ServiceProcess("mqtt-iot");
        mqtt_iot.endpoint("POST", "/sensors/data", "receive_sensor_data");
        mqtt_iot.endpoint("GET", "/sensors", "list_sensors");
        mqtt_iot.endpoint("GET", "/mqtt/config", "mqtt_config");
        mqtt_iot.endpoint("GET", "/mqtt/topics", "mqtt_topics");
        server.service(mqtt_iot);
        ServiceProcess root = new ServiceProcess("root");
        server.service(root);
        server.compile();
    }
}
