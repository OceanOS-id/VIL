// ╔════════════════════════════════════════════════════════════╗
// ║  015 — Smart Factory IoT Gateway (MQTT)                   ║
// ╠════════════════════════════════════════════════════════════╣
// ║  Pattern:  VX_APP                                           ║
// ║  Token:    N/A (HTTP server)                                ║
// ║  Features: ShmSlice, ServiceCtx, VilResponse                ║
// ║  Domain:   Industrial sensor data from factory floor —      ║
// ║            temperature, vibration, pressure, quality metrics ║
// ╚════════════════════════════════════════════════════════════╝
// basic-usage-mqtt-iot-gateway — MQTT Sensors → REST API Bridge (VX Process-Oriented)
// =============================================================================
//
// BUSINESS CONTEXT:
//   Smart factory IoT gateway for Industry 4.0 manufacturing. Sensors on the
//   factory floor report temperature (oven zones), vibration (CNC machines),
//   pressure (hydraulic presses), and motion (safety zones). The gateway
//   bridges sensor data from lightweight MQTT to the analytics platform via
//   Tri-Lane SHM. Alerts trigger when readings exceed thresholds — e.g.,
//   vibration > 4mm/s on a bearing signals predictive maintenance needed.
//   QoS AtLeastOnce ensures no sensor reading is lost (regulatory requirement).
//
// Demonstrates vil_mq_mqtt integration for an IoT gateway pattern using
// the VX Process-Oriented architecture (VilApp + ServiceProcess): REST
// endpoints receive sensor data and bridge it to MQTT topics, while MQTT
// subscriptions feed back into the REST API. The MQTT client uses an
// in-memory implementation, so this example runs without a real MQTT broker.
//
// Features demonstrated:
//   - MqttConfig — broker connection, QoS levels, TLS, keepalive
//   - MqttClient — publish, subscribe, connection management
//   - MqttBridge — MQTT → Tri-Lane SHM bridge for inter-service delivery
//   - IoT gateway pattern: REST ↔ MQTT bidirectional bridge
//
// Routes:
//   GET  /                    → overview of IoT gateway
//   POST /api/sensors/data    → receive sensor data via REST, publish to MQTT
//   GET  /api/sensors         → list registered sensors (seed data)
//   GET  /api/mqtt/config     → MQTT broker connection status
//   GET  /api/mqtt/topics     → subscribed MQTT topics
//
// Built-in endpoints (auto-provided by VilServer):
//   GET  /health, /ready, /metrics, /info
//
// Run:
//   cargo run -p basic-usage-mqtt-iot-gateway
//
// Test:
//   curl http://localhost:8080/
//   curl -X POST http://localhost:8080/api/sensors/data \
//     -H 'Content-Type: application/json' \
//     -d '{"sensor_id":"temp-001","type":"temperature","value":23.5,"unit":"celsius"}'
//   curl http://localhost:8080/api/sensors
//   curl http://localhost:8080/api/mqtt/config
//   curl http://localhost:8080/api/mqtt/topics
// =============================================================================

use vil_server::prelude::*;
use vil_server::axum::extract::Extension;

use vil_mq_mqtt::{MqttConfig, MqttClient, QoS, MqttBridge};

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

// ---------------------------------------------------------------------------
// Domain types
// ---------------------------------------------------------------------------

/// Sensor reading received via REST.
#[derive(Debug, Clone, Serialize, Deserialize, VilModel)]
struct SensorReading {
    sensor_id: String,
    #[serde(rename = "type")]
    sensor_type: String,
    value: f64,
    unit: String,
    #[serde(default)]
    timestamp: Option<String>,
}

/// Registered sensor with metadata and last reading.
#[derive(Debug, Clone, Serialize, Deserialize, VilModel)]
struct SensorInfo {
    sensor_id: String,
    sensor_type: String,
    location: String,
    mqtt_topic: String,
    last_value: Option<f64>,
    last_unit: Option<String>,
    readings_count: u64,
}

// ---------------------------------------------------------------------------
// Response types
// ---------------------------------------------------------------------------

#[derive(Clone, Debug, Serialize, Deserialize, VilModel)]
struct SensorDataResponse {
    status: String,
    mqtt_topic: String,
    qos: String,
    payload_size: usize,
    total_published: u64,
    bridged_to_shm: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize, VilModel)]
struct SensorListResponse {
    pre_registered_sensors: Vec<SensorInfo>,
    live_sensors: Vec<SensorInfo>,
    total_pre_registered: usize,
    total_live: usize,
    note: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, VilModel)]
struct MqttConnectionInfo {
    broker_url: String,
    port: u16,
    client_id: Option<String>,
    connected: bool,
    tls: bool,
    keepalive_secs: u64,
    qos: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, VilModel)]
struct MqttMetrics {
    messages_published: u64,
    messages_received: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize, VilModel)]
struct MqttBridgeInfo {
    target_service: String,
    messages_bridged: u64,
    description: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, VilModel)]
struct QosLevels {
    at_most_once: String,
    at_least_once: String,
    exactly_once: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, VilModel)]
struct MqttConfigResponse {
    connection: MqttConnectionInfo,
    metrics: MqttMetrics,
    bridge: MqttBridgeInfo,
    qos_levels: QosLevels,
    note: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, VilModel)]
struct TopicPattern {
    pattern: String,
    description: String,
    #[serde(default)]
    example_matches: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, VilModel)]
struct GatewayPattern {
    description: String,
    inbound: String,
    outbound: String,
    bridge: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, VilModel)]
struct MqttTopicsResponse {
    subscribed_topics: Vec<String>,
    topic_patterns: Vec<TopicPattern>,
    gateway_pattern: GatewayPattern,
}

// ---------------------------------------------------------------------------
// Shared state
// ---------------------------------------------------------------------------

#[derive(Clone)]
struct MqttState {
    client: Arc<MqttClient>,
    bridge: Arc<MqttBridge>,
    config: MqttConfig,
    sensors: Arc<RwLock<HashMap<String, SensorInfo>>>,
    subscribed_topics: Arc<Vec<String>>,
}

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

/// GET / — overview of the IoT gateway.
async fn index() -> &'static str {
    "VIL MQTT IoT Gateway Example\n\
     ==================================\n\n\
     Demonstrates an IoT gateway: REST ↔ MQTT bidirectional bridge.\n\n\
     Endpoints:\n\
     - POST /api/sensors/data   — send sensor data (REST → MQTT)\n\
     - GET  /api/sensors        — list registered sensors with last readings\n\
     - GET  /api/mqtt/config    — MQTT broker connection status\n\
     - GET  /api/mqtt/topics    — subscribed MQTT topics\n\n\
     Built-in:\n\
     - GET  /health, /ready, /metrics, /info\n"
}

/// POST /api/sensors/data — receive sensor data via REST and publish to MQTT.
async fn receive_sensor_data(
    ctx: ServiceCtx,
    body: ShmSlice,
) -> HandlerResult<VilResponse<SensorDataResponse>> {
    let state = ctx.state::<MqttState>().expect("state type mismatch");
    let reading: SensorReading = body.json().expect("invalid JSON body");
    // Validate sensor_id
    if reading.sensor_id.trim().is_empty() {
        return Err(VilError::bad_request("sensor_id must not be empty"));
    }

    // Build the MQTT topic for this sensor.
    // Topic hierarchy: sensors/{type}/{id} enables wildcard subscriptions —
    // e.g., "sensors/temperature/+" subscribes to ALL temperature sensors.
    let mqtt_topic = format!("sensors/{}/{}", reading.sensor_type, reading.sensor_id);

    // Serialize payload for MQTT
    let payload = serde_json::to_vec(&reading)
        .map_err(|e| VilError::internal(format!("Serialization failed: {}", e)))?;

    // Publish to MQTT (implementation mode)
    state.client.publish(&mqtt_topic, &payload, QoS::AtLeastOnce).await
        .map_err(|e| VilError::internal(format!("MQTT publish failed: {}", e)))?;

    // Bridge to Tri-Lane SHM
    state.bridge.bridge(&mqtt_topic, &payload).await;

    // Update sensor registry
    {
        let mut sensors = state.sensors.write().await;
        let entry = sensors.entry(reading.sensor_id.clone()).or_insert_with(|| SensorInfo {
            sensor_id: reading.sensor_id.clone(),
            sensor_type: reading.sensor_type.clone(),
            location: "unknown".into(),
            mqtt_topic: mqtt_topic.clone(),
            last_value: None,
            last_unit: None,
            readings_count: 0,
        });
        entry.last_value = Some(reading.value);
        entry.last_unit = Some(reading.unit.clone());
        entry.readings_count += 1;
    }

    Ok(VilResponse::ok(SensorDataResponse {
        status: "published".into(),
        mqtt_topic,
        qos: "AtLeastOnce".into(),
        payload_size: payload.len(),
        total_published: state.client.published_count(),
        bridged_to_shm: true,
    }))
}

/// GET /api/sensors — list registered sensors with seed and live data.
async fn list_sensors(
    ctx: ServiceCtx,
) -> VilResponse<SensorListResponse> {
    let state = ctx.state::<MqttState>().expect("state type mismatch");
    let sensors = state.sensors.read().await;
    let live_sensors: Vec<SensorInfo> = sensors.values().cloned().collect();

    // Combine pre-registered sensors with any live data
    let seed_sensors = vec![
        SensorInfo {
            sensor_id: "temp-001".into(),
            sensor_type: "temperature".into(),
            location: "Building A, Floor 2, Room 201".into(),
            mqtt_topic: "sensors/temperature/temp-001".into(),
            last_value: Some(22.5),
            last_unit: Some("celsius".into()),
            readings_count: 1842,
        },
        SensorInfo {
            sensor_id: "hum-003".into(),
            sensor_type: "humidity".into(),
            location: "Building A, Floor 2, Room 201".into(),
            mqtt_topic: "sensors/humidity/hum-003".into(),
            last_value: Some(45.2),
            last_unit: Some("percent".into()),
            readings_count: 1840,
        },
        SensorInfo {
            sensor_id: "pres-007".into(),
            sensor_type: "pressure".into(),
            location: "Building B, Roof".into(),
            mqtt_topic: "sensors/pressure/pres-007".into(),
            last_value: Some(1013.25),
            last_unit: Some("hPa".into()),
            readings_count: 920,
        },
        SensorInfo {
            sensor_id: "motion-012".into(),
            sensor_type: "motion".into(),
            location: "Building A, Floor 1, Entrance".into(),
            mqtt_topic: "sensors/motion/motion-012".into(),
            last_value: Some(1.0),
            last_unit: Some("boolean".into()),
            readings_count: 4510,
        },
    ];

    let total_pre_registered = seed_sensors.len();
    let total_live = live_sensors.len();

    VilResponse::ok(SensorListResponse {
        pre_registered_sensors: seed_sensors,
        live_sensors,
        total_pre_registered,
        total_live,
        note: "Pre-registered sensors are seed data. Live sensors appear after POST /api/sensors/data.".into(),
    })
}

/// GET /api/mqtt/config — MqttConfig with QoS levels and connection fields.
async fn mqtt_config(
    ctx: ServiceCtx,
) -> VilResponse<MqttConfigResponse> {
    let state = ctx.state::<MqttState>().expect("state type mismatch");
    VilResponse::ok(MqttConfigResponse {
        connection: MqttConnectionInfo {
            broker_url: state.config.broker_url.clone(),
            port: state.config.port,
            client_id: state.config.client_id.clone(),
            connected: state.client.is_connected(),
            tls: state.config.tls,
            keepalive_secs: state.config.keepalive_secs,
            qos: format!("{:?}", state.config.qos),
        },
        metrics: MqttMetrics {
            messages_published: state.client.published_count(),
            messages_received: state.client.received_count(),
        },
        bridge: MqttBridgeInfo {
            target_service: "sensor-analytics".into(),
            messages_bridged: state.bridge.bridged_count(),
            description: "MqttBridge forwards MQTT messages to Tri-Lane SHM mesh".into(),
        },
        qos_levels: QosLevels {
            at_most_once: "Fire and forget — no acknowledgment".into(),
            at_least_once: "Guaranteed delivery — may have duplicates (default)".into(),
            exactly_once: "Guaranteed exactly once — highest overhead".into(),
        },
        note: "Stub mode — no real MQTT broker connected.".into(),
    })
}

/// GET /api/mqtt/topics — list subscribed MQTT topics.
async fn mqtt_topics(
    ctx: ServiceCtx,
) -> VilResponse<MqttTopicsResponse> {
    let state = ctx.state::<MqttState>().expect("state type mismatch");
    VilResponse::ok(MqttTopicsResponse {
        subscribed_topics: (*state.subscribed_topics).clone(),
        topic_patterns: vec![
            TopicPattern {
                pattern: "sensors/+/+".into(),
                description: "All sensor readings (wildcard: type and sensor_id)".into(),
                example_matches: vec![
                    "sensors/temperature/temp-001".into(),
                    "sensors/humidity/hum-003".into(),
                    "sensors/pressure/pres-007".into(),
                ],
            },
            TopicPattern {
                pattern: "sensors/temperature/#".into(),
                description: "All temperature sensor readings (multi-level wildcard)".into(),
                example_matches: vec![
                    "sensors/temperature/temp-001".into(),
                    "sensors/temperature/building-a/floor-2".into(),
                ],
            },
            TopicPattern {
                pattern: "commands/+/+".into(),
                description: "Device control commands (type and device_id)".into(),
                example_matches: vec![
                    "commands/hvac/unit-001".into(),
                    "commands/lighting/zone-a".into(),
                ],
            },
            TopicPattern {
                pattern: "$SYS/#".into(),
                description: "Broker system topics (connection stats, etc.)".into(),
                example_matches: vec![],
            },
        ],
        gateway_pattern: GatewayPattern {
            description: "IoT Gateway: REST ↔ MQTT bidirectional bridge".into(),
            inbound: "REST POST /api/sensors/data → MQTT publish to sensors/{type}/{id}".into(),
            outbound: "MQTT subscribe sensors/+/+ → internal processing → REST GET /api/sensors".into(),
            bridge: "MQTT → MqttBridge → Tri-Lane SHM → analytics-service".into(),
        },
    })
}

// ---------------------------------------------------------------------------
// Main — VX Process-Oriented (VilApp + ServiceProcess)
// ---------------------------------------------------------------------------

#[tokio::main]
async fn main() {
    // Configure MQTT connection
    let mqtt_cfg = MqttConfig::new("mqtt://localhost")
        .client_id("vil-iot-gateway")
        .qos(QoS::AtLeastOnce);

    // Create MQTT client (implementation mode)
    let client = MqttClient::new(mqtt_cfg.clone()).await
        .expect("MQTT client creation should succeed (implementation mode)");

    // Subscribe to sensor topics using MQTT wildcards:
    //   sensors/+/+       — all sensor readings (for real-time dashboard)
    //   sensors/temperature/# — temperature only (for oven zone monitoring)
    //   commands/+/+      — device control commands (HVAC, lighting)
    let topics = vec![
        "sensors/+/+".to_string(),
        "sensors/temperature/#".to_string(),
        "commands/+/+".to_string(),
    ];
    for topic in &topics {
        let _ = client.subscribe(topic).await;
    }

    // Create bridge to Tri-Lane SHM
    let bridge = MqttBridge::new("sensor-analytics");

    // Pre-register some sensors
    let sensors: HashMap<String, SensorInfo> = HashMap::new();

    let state = MqttState {
        client: Arc::new(client),
        bridge: Arc::new(bridge),
        config: mqtt_cfg,
        sensors: Arc::new(RwLock::new(sensors)),
        subscribed_topics: Arc::new(topics),
    };

    // ── Step 2: Define the MQTT IoT service as a Process ─────────────
    let mqtt_service = ServiceProcess::new("mqtt-iot")
        .prefix("/api")
        .endpoint(Method::POST, "/sensors/data", post(receive_sensor_data))
        .endpoint(Method::GET,  "/sensors",      get(list_sensors))
        .endpoint(Method::GET,  "/mqtt/config",  get(mqtt_config))
        .endpoint(Method::GET,  "/mqtt/topics",  get(mqtt_topics))
        .state(state);

    // ── Step 3: Assemble into VilApp and run ───────────────────────
    VilApp::new("mqtt-iot-gateway")
        .port(8080)
        .service(ServiceProcess::new("root")
            .endpoint(Method::GET, "/", get(index)))
        .service(mqtt_service)
        .run()
        .await;
}
