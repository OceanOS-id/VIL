# Protocol Connectors

VIL native connectors for legacy/industrial protocols and WebSocket server.

## Quick Reference

| Connector | Crate | Use Case |
|-----------|-------|----------|
| SOAP | vil_conn_soap | Enterprise web services / legacy XML |
| OPC-UA | vil_conn_opcua | Industrial automation, SCADA |
| Modbus | vil_conn_modbus | Industrial control systems, PLCs |
| WebSocket Server | vil_conn_ws | Real-time bidirectional streaming |

## SOAP (vil_conn_soap)

```rust
use vil_conn_soap::{SoapConnector, SoapConfig};

let soap = SoapConnector::new(SoapConfig {
    wsdl_url: "http://legacy-service/api?wsdl".into(),
    endpoint: "http://legacy-service/api".into(),
    ..Default::default()
}).await?;

// Call SOAP operation
let response: GetOrderResponse = soap
    .call("GetOrder", GetOrderRequest { order_id: 123 })
    .await?;

// With basic auth
let soap_auth = SoapConnector::new(SoapConfig {
    endpoint: "http://legacy-service/api".into(),
    username: Some("user".into()),
    password: Some("pass".into()),
    ..Default::default()
}).await?;
```

### Raw envelope (advanced)
```rust
let raw_response = soap.call_raw(r#"
    <soapenv:Envelope xmlns:soapenv="http://schemas.xmlsoap.org/soap/envelope/">
        <soapenv:Body>
            <GetOrder><OrderId>123</OrderId></GetOrder>
        </soapenv:Body>
    </soapenv:Envelope>
"#).await?;
```

### VilApp bridge
```rust
#[vil_handler(shm)]
async fn get_order_from_legacy(ctx: ServiceCtx, Path(id): Path<i64>) -> VilResponse<Order> {
    let soap = ctx.state::<SoapConnector>();
    let resp: GetOrderResponse = soap.call("GetOrder", GetOrderRequest { order_id: id }).await?;
    VilResponse::ok(Order::from(resp))
}
```

## OPC-UA (vil_conn_opcua)

For industrial automation, SCADA, and IoT sensor networks.

```rust
use vil_conn_opcua::{OpcUaConnector, OpcUaConfig, NodeId};

let opcua = OpcUaConnector::new(OpcUaConfig {
    endpoint: "opc.tcp://plc-server:4840/".into(),
    security_policy: SecurityPolicy::None,
    ..Default::default()
}).await?;

// Read node value
let temp: f64 = opcua.read(NodeId::new(2, "Temperature")).await?;

// Write node value
opcua.write(NodeId::new(2, "Setpoint"), 75.0_f64).await?;

// Subscribe to changes
opcua.subscribe(vec![
    NodeId::new(2, "Temperature"),
    NodeId::new(2, "Pressure"),
], |change: NodeChange| async move {
    app_log!(Info, "opcua.value_change", {
        node: change.node_id.to_string(),
        value: change.value.to_string()
    });
    Ok(())
}).await?;
```

### Browse namespace
```rust
let nodes = opcua.browse(NodeId::root()).await?;
for node in nodes {
    println!("{}: {:?}", node.id, node.display_name);
}
```

## Modbus (vil_conn_modbus)

For PLCs, VFDs, and industrial control systems.

```rust
use vil_conn_modbus::{ModbusConnector, ModbusConfig, ModbusMode};

// TCP (most common)
let modbus = ModbusConnector::new(ModbusConfig {
    host: "192.168.1.100".into(),
    port: 502,
    mode: ModbusMode::Tcp,
    unit_id: 1,
    ..Default::default()
}).await?;

// Read holding registers
let registers: Vec<u16> = modbus.read_holding_registers(0, 10).await?;

// Read input registers (read-only sensors)
let inputs: Vec<u16> = modbus.read_input_registers(100, 5).await?;

// Read coils (discrete output)
let coils: Vec<bool> = modbus.read_coils(0, 8).await?;

// Write single register
modbus.write_single_register(10, 1500_u16).await?;

// Write multiple registers
modbus.write_multiple_registers(10, &[1500_u16, 2000, 500]).await?;
```

### RTU (serial)
```rust
ModbusConfig {
    port: 0,   // unused for RTU
    mode: ModbusMode::Rtu {
        serial_port: "/dev/ttyUSB0".into(),
        baud_rate: 9600,
        parity: Parity::None,
    },
    unit_id: 1,
    ..Default::default()
}
```

### Polling loop to VIL pipeline
```rust
let modbus = modbus.clone();
tokio::spawn(async move {
    loop {
        let regs = modbus.read_holding_registers(0, 10).await?;
        let reading = SensorReading::from_registers(&regs);
        app_log!(Info, "modbus.reading", { values: format!("{:?}", regs) });
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
});
```

## WebSocket Server (vil_conn_ws)

Standalone WebSocket server with topic-based pub/sub.

```rust
use vil_conn_ws::{WsServer, WsServerConfig, WsMessage};

let ws_server = WsServer::new(WsServerConfig {
    port: 9090,
    max_connections: 10_000,
    ..Default::default()
}).build()?;

// Broadcast to all clients
ws_server.broadcast(WsMessage::text("hello everyone")).await?;

// Broadcast to topic subscribers
ws_server.broadcast_topic("prices", &price_update).await?;

// Handle incoming messages
ws_server.on_message(|client_id, msg: WsMessage| async move {
    if let Ok(cmd) = msg.json::<Command>() {
        ws_server.send(client_id, &process_command(cmd)).await?;
    }
    Ok(())
}).await?;
```

### VilApp integration (WebSocket + REST)
```rust
let ws = Arc::new(WsServer::new(WsServerConfig { port: 9090, ..Default::default() }).build()?);

let service = ServiceProcess::new("api")
    .extension(ws.clone())
    .endpoint(Method::POST, "/publish/:topic", post(publish_to_ws));

#[vil_handler(shm)]
async fn publish_to_ws(ctx: ServiceCtx, Path(topic): Path<String>, slice: ShmSlice) -> VilResponse<()> {
    let ws = ctx.state::<Arc<WsServer>>();
    ws.broadcast_topic(&topic, slice.bytes()).await?;
    VilResponse::ok(())
}
```

Note: For WebSocket as a VilApp service endpoint (not standalone), see [integrations/websocket.md](../integrations/websocket.md).
