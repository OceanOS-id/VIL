// =============================================================================
// vil_modbus::config — ModbusConfig
// =============================================================================
//
// Configuration for the Modbus TCP/RTU client.
// External layout profile acceptable for setup-time data (COMPLIANCE.md §4).
// =============================================================================

/// Modbus transport mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModbusTransport {
    /// Modbus over TCP (most common for modern PLCs and gateways).
    Tcp,
    /// Modbus RTU framing over a serial port (legacy devices).
    Rtu,
}

/// Configuration for the VIL Modbus client.
///
/// # Example YAML
/// ```yaml
/// modbus:
///   host: "192.168.1.10"
///   port: 502
///   unit_id: 1
///   transport: Tcp
///   timeout_ms: 1000
/// ```
#[derive(Debug, Clone)]
pub struct ModbusConfig {
    /// Modbus server hostname or IP address.
    pub host: String,
    /// Modbus TCP port (default: 502).
    pub port: u16,
    /// Modbus unit/slave ID (device address).
    pub unit_id: u8,
    /// Transport mode (TCP or RTU).
    pub transport: ModbusTransport,
    /// Request timeout in milliseconds.
    pub timeout_ms: u64,
}

impl ModbusConfig {
    /// Construct a new `ModbusConfig` with defaults.
    pub fn new(host: impl Into<String>, port: u16, unit_id: u8) -> Self {
        Self {
            host: host.into(),
            port,
            unit_id,
            transport: ModbusTransport::Tcp,
            timeout_ms: 1_000,
        }
    }

    /// Override the transport mode.
    pub fn with_transport(mut self, transport: ModbusTransport) -> Self {
        self.transport = transport;
        self
    }

    /// Override the timeout.
    pub fn with_timeout_ms(mut self, ms: u64) -> Self {
        self.timeout_ms = ms;
        self
    }

    /// Return the socket address string "host:port".
    pub fn socket_addr(&self) -> String {
        format!("{}:{}", self.host, self.port)
    }
}
