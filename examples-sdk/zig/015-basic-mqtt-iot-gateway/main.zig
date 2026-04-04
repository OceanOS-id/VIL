// 015-basic-mqtt-iot-gateway — Zig SDK equivalent
// Compile: vil compile --from zig --input 015-basic-mqtt-iot-gateway/main.zig --release

const vil = @import("vil");

pub fn main() void {
    var server = vil.Server.init("mqtt-iot-gateway", 8080);
    var mqtt_iot = vil.Service.init("mqtt-iot");
    mqtt_iot.endpoint("POST", "/sensors/data", "receive_sensor_data");
    mqtt_iot.endpoint("GET", "/sensors", "list_sensors");
    mqtt_iot.endpoint("GET", "/mqtt/config", "mqtt_config");
    mqtt_iot.endpoint("GET", "/mqtt/topics", "mqtt_topics");
    server.service(&mqtt_iot);
    var root = vil.Service.init("root");
    server.service(&root);
    server.compile();
}
