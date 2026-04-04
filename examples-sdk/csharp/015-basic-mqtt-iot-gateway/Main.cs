// 015-basic-mqtt-iot-gateway — C# SDK equivalent
// Compile: vil compile --from csharp --input 015-basic-mqtt-iot-gateway/Main.cs --release

#r "sdk/csharp/Vil.cs"

var server = new VilServer("mqtt-iot-gateway", 8080);
var mqtt_iot = new ServiceProcess("mqtt-iot");
mqtt_iot.Endpoint("POST", "/sensors/data", "receive_sensor_data");
mqtt_iot.Endpoint("GET", "/sensors", "list_sensors");
mqtt_iot.Endpoint("GET", "/mqtt/config", "mqtt_config");
mqtt_iot.Endpoint("GET", "/mqtt/topics", "mqtt_topics");
server.Service(mqtt_iot);
var root = new ServiceProcess("root");
server.Service(root);
server.Compile();
