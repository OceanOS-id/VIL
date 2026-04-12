// 015-basic-mqtt-iot-gateway — MQTT IoT Gateway (VWFD)
//
// Endpoints:
//   GET  /api/mqtt/config  → MQTT config
//   GET  /api/sensors      → sensor list
//   GET  /api/mqtt/topics  → MQTT topics
//   POST /api/sensors/data → publish sensor data (returns "published" + "mqtt_topic")

use serde_json::{json, Value};

fn mqtt_config(_input: &Value) -> Result<Value, String> {
    Ok(json!({
        "mqtt": {
            "broker": "localhost:19883",
            "protocol": "mqtt",
            "clean_session": true,
            "keepalive_secs": 60
        },
        "qos": 1,
        "status": "connected"
    }))
}

fn list_sensors(_input: &Value) -> Result<Value, String> {
    Ok(json!({
        "sensors": [
            {"id": "temp-001", "type": "temperature", "unit": "celsius", "status": "active"},
            {"id": "hum-001", "type": "humidity", "unit": "percent", "status": "active"},
            {"id": "pres-001", "type": "pressure", "unit": "hPa", "status": "active"}
        ],
        "total": 3
    }))
}

fn mqtt_topics(_input: &Value) -> Result<Value, String> {
    Ok(json!({
        "topics": [
            "sensors/temperature/#",
            "sensors/humidity/#",
            "sensors/pressure/#",
            "sensors/+/data"
        ],
        "subscribed": 4
    }))
}

fn publish_sensor_data(input: &Value) -> Result<Value, String> {
    let sensor_id = input.get("body")
        .and_then(|b| b["sensor_id"].as_str())
        .unwrap_or("unknown");
    let sensor_type = input.get("body")
        .and_then(|b| b["type"].as_str())
        .unwrap_or("generic");
    Ok(json!({
        "published": true,
        "mqtt_topic": format!("sensors/{}/{}", sensor_type, sensor_id),
        "qos": 1,
        "retained": false
    }))
}

#[tokio::main]
async fn main() {
    vil_vwfd::app("examples/015-basic-mqtt-iot-gateway/vwfd/workflows", 8080)
        .native("mqtt_config", mqtt_config)
        .native("list_sensors", list_sensors)
        .native("mqtt_topics", mqtt_topics)
        .native("publish_sensor_data", publish_sensor_data)
        .run()
        .await;
}
