// =============================================================================
// example-704-protocol-modbus-read — Modbus TCP register read
// =============================================================================
//
// Demonstrates:
//   - ModbusClient::connect() to a Modbus TCP server
//   - read_registers() for holding registers
//   - read_coils() for coil (digital output) status
//   - db_log! auto-emitted on every read operation
//   - StdoutDrain::resolved() output
//
// Requires: A Modbus TCP server running locally.
// Quick start (Modbus simulator):
//   docker run -p 502:502 oitc/modbus-server
//
// Without a Modbus server, this example prints config and exits gracefully.
// =============================================================================

use vil_log::drain::{StdoutDrain, StdoutFormat};
use vil_log::runtime::init_logging;
use vil_log::{LogConfig, LogLevel};
use vil_modbus::{ModbusClient, ModbusConfig};

const MODBUS_HOST: &str = "127.0.0.1";
const MODBUS_PORT: u16 = 502;
const UNIT_ID: u8 = 1;

#[tokio::main]
async fn main() {
    // ── Init vil_log with resolved drain ──
    let config = LogConfig {
        ring_slots: 4096,
        level: LogLevel::Info,
        batch_size: 64,
        flush_interval_ms: 50,
        threads: None,
        dict_path: None,
        fallback_path: None,
        drain_failure_threshold: 3,
    };
    let _task = init_logging(config, StdoutDrain::new(StdoutFormat::Resolved));

    println!();
    println!("  example-704-protocol-modbus-read");
    println!("  Modbus TCP register read with db_log! auto-emit");
    println!();

    let modbus_cfg = ModbusConfig::new(MODBUS_HOST, MODBUS_PORT, UNIT_ID);

    println!("  Host:    {}:{}", MODBUS_HOST, MODBUS_PORT);
    println!("  Unit ID: {}", UNIT_ID);
    println!();
    println!("  NOTE: Requires a Modbus TCP server running locally.");
    println!("  Quick start with a simulator:");
    println!("    docker run -p 502:502 oitc/modbus-server");
    println!();

    let mut client = match ModbusClient::connect(modbus_cfg).await {
        Ok(c) => c,
        Err(e) => {
            println!("  [SKIP] Cannot connect to Modbus server: {:?}", e);
            println!("  (All db_log! calls would appear above in resolved format)");
            println!();
            println!("  In production, a successful read_registers call emits:");
            println!(
                "    db_log! {{ op_type=0(SELECT), db_hash=<host:port>, rows_affected=<count> }}"
            );
            return;
        }
    };

    // ── Read holding registers 0..9 (10 registers) ──
    println!("  Reading holding registers 0..9...");
    match client.read_registers(0, 10).await {
        Ok(values) => {
            println!("  REGISTERS[0..9] = {:?}", values);
        }
        Err(e) => println!("  read_registers error: {:?}", e),
    }

    // ── Read coils 0..7 (8 coils) ──
    println!("  Reading coils 0..7...");
    match client.read_coils(0, 8).await {
        Ok(values) => {
            let bits: Vec<u8> = values.iter().map(|b| *b as u8).collect();
            println!("  COILS[0..7] = {:?}", bits);
        }
        Err(e) => println!("  read_coils error: {:?}", e),
    }

    // Allow drain to flush
    tokio::time::sleep(std::time::Duration::from_millis(200)).await;
    println!();
    println!("  Done. db_log! entries (op_type=0 SELECT) emitted above.");
    println!();
}
