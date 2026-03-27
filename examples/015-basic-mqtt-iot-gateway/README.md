# 015 — MQTT IoT Gateway

IoT gateway pattern with REST-to-MQTT bidirectional bridge, sensor registry, and MqttBridge to Tri-Lane SHM.

| Property | Value |
|----------|-------|
| **Pattern** | VX_APP |
| **Token** | N/A |
| **Body** | ShmSlice (zero-copy) |
| **Context** | ServiceCtx (Tri-Lane) |
| **Transform** | N/A |

## Architecture

```
POST /api/sensors/data, GET /api/sensors, GET /api/mqtt/config, GET /api/mqtt/topics
```

## Key VIL Features Used

- `MqttClient with QoS levels`
- `MqttBridge to Tri-Lane SHM`
- `ShmSlice for sensor data body`
- `ServiceCtx with shared sensor registry`
- `VilResponse + VilModel`

## Run

```bash
cargo run -p basic-usage-mqtt-iot-gateway
```

## Test

```bash
curl -X POST http://localhost:8080/api/sensors/data -H 'Content-Type: application/json' -d '{"sensor_id":"temp-001","type":"temperature","value":23.5,"unit":"celsius"}'
```
