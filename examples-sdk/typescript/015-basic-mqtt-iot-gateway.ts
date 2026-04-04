#!/usr/bin/env tsx
// 015-basic-mqtt-iot-gateway — TypeScript SDK equivalent
// Compile: vil compile --from typescript --input 015-basic-mqtt-iot-gateway.ts --release

import { VilPipeline, VilServer, ServiceProcess } from "vil-sdk";

const server = new VilServer("mqtt-iot-gateway", 8080);
const mqtt_iot = new ServiceProcess("mqtt-iot");
mqtt_iot.endpoint("POST", "/sensors/data", "receive_sensor_data");
mqtt_iot.endpoint("GET", "/sensors", "list_sensors");
mqtt_iot.endpoint("GET", "/mqtt/config", "mqtt_config");
mqtt_iot.endpoint("GET", "/mqtt/topics", "mqtt_topics");
server.service(mqtt_iot);
const root = new ServiceProcess("root");
server.service(root);
server.compile();
