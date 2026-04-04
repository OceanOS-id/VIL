#!/usr/bin/env python3
"""015-basic-mqtt-iot-gateway — Python SDK equivalent
Compile: vil compile --from python --input 015-basic-mqtt-iot-gateway.py --release
"""
import os
from vil import VilPipeline, VilServer, ServiceProcess

server = VilServer("mqtt-iot-gateway", port=8080)
mqtt_iot = server.service_process("mqtt-iot")
mqtt_iot.endpoint("POST", "/sensors/data", "receive_sensor_data")
mqtt_iot.endpoint("GET", "/sensors", "list_sensors")
mqtt_iot.endpoint("GET", "/mqtt/config", "mqtt_config")
mqtt_iot.endpoint("GET", "/mqtt/topics", "mqtt_topics")
root = server.service_process("root")
server.compile()
