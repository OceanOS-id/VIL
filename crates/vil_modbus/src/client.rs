// =============================================================================
// vil_modbus::client — ModbusClient
// =============================================================================
//
// Modbus TCP/RTU client with VIL semantic log integration.
//
// - read_coils()      emits db_log! (op_type=0 SELECT) with timing.
// - read_registers()  emits db_log! (op_type=0 SELECT) with timing.
// - write_coil()      emits db_log! (op_type=2 UPDATE) with timing.
// - write_register()  emits db_log! (op_type=2 UPDATE) with timing.
// - No println!, tracing::info!, or log::info! — COMPLIANCE.md §8.
// - String fields use register_str() hashes — no raw strings on hot path.
//
// Note: tokio-modbus returns Result<Result<T, Exception>, Error>.
// The inner Result<T, Exception> holds the Modbus-layer response code.
// =============================================================================

use tokio_modbus::client::tcp;
use tokio_modbus::client::Context;
use tokio_modbus::prelude::*;

use vil_log::dict::register_str;
use vil_log::{db_log, types::DbPayload};

use crate::config::ModbusConfig;
use crate::error::ModbusFault;

/// Modbus TCP/RTU client with integrated VIL semantic logging.
///
/// Every operation automatically emits a `db_log!` entry with:
/// - `db_hash`       — FxHash of the "host:port" connection string
/// - `table_hash`    — FxHash of the register address as decimal string
/// - `duration_ns`   — Wall-clock time of the operation
/// - `rows_affected` — Number of coils/registers read or written
/// - `op_type`       — 0=read (SELECT), 2=write (UPDATE)
/// - `error_code`    — 0 on success, non-zero on fault
///
/// Thread hint: ModbusClient uses tokio-modbus async internally.
/// No extra log threads spawned — add 0 to `LogConfig.threads`.
pub struct ModbusClient {
    ctx: Context,
    /// Cached FxHash of the "host:port" string.
    host_hash: u32,
    config: ModbusConfig,
}

impl ModbusClient {
    /// Connect to a Modbus TCP server and return a ready `ModbusClient`.
    pub async fn connect(config: ModbusConfig) -> Result<Self, ModbusFault> {
        let socket_addr_str = config.socket_addr();
        let host_hash = register_str(&socket_addr_str);

        let socket_addr: std::net::SocketAddr =
            socket_addr_str
                .parse()
                .map_err(|_| ModbusFault::ConnectionFailed {
                    host_hash,
                    reason_code: 1,
                })?;

        let ctx = tcp::connect_slave(socket_addr, Slave(config.unit_id))
            .await
            .map_err(|e: std::io::Error| ModbusFault::ConnectionFailed {
                host_hash,
                reason_code: e.raw_os_error().unwrap_or(0) as u32,
            })?;

        Ok(Self {
            ctx,
            host_hash,
            config,
        })
    }

    /// Read `count` coils starting at `address`.
    ///
    /// Emits `db_log!` (op_type=0 SELECT) with timing.
    pub async fn read_coils(&mut self, address: u16, count: u16) -> Result<Vec<bool>, ModbusFault> {
        let start = std::time::Instant::now();
        let addr_str = format!("coil:{}", address);
        let addr_hash = register_str(&addr_str);

        let raw = self.ctx.read_coils(address, count).await;
        let result: Result<Vec<bool>, ModbusFault> = match raw {
            Err(_) => Err(ModbusFault::ReadCoilsFailed {
                address,
                exception_code: 0,
            }),
            Ok(Err(ex)) => Err(ModbusFault::ReadCoilsFailed {
                address,
                exception_code: ex as u8,
            }),
            Ok(Ok(v)) => Ok(v),
        };

        let elapsed = start.elapsed();
        let (rows, err_code) = match &result {
            Ok(v) => (v.len() as u32, 0u8),
            Err(f) => (0, f.as_error_code()),
        };

        db_log!(
            Info,
            DbPayload {
                db_hash: self.host_hash,
                table_hash: addr_hash,
                query_hash: register_str("read_coils"),
                duration_ns: elapsed.as_nanos() as u64,
                rows_affected: rows,
                op_type: 0, // SELECT / read
                error_code: err_code,
                ..DbPayload::default()
            }
        );

        result
    }

    /// Read `count` holding registers starting at `address`.
    ///
    /// Emits `db_log!` (op_type=0 SELECT) with timing.
    pub async fn read_registers(
        &mut self,
        address: u16,
        count: u16,
    ) -> Result<Vec<u16>, ModbusFault> {
        let start = std::time::Instant::now();
        let addr_str = format!("reg:{}", address);
        let addr_hash = register_str(&addr_str);

        let raw = self.ctx.read_holding_registers(address, count).await;
        let result: Result<Vec<u16>, ModbusFault> = match raw {
            Err(_) => Err(ModbusFault::ReadRegistersFailed {
                address,
                exception_code: 0,
            }),
            Ok(Err(ex)) => Err(ModbusFault::ReadRegistersFailed {
                address,
                exception_code: ex as u8,
            }),
            Ok(Ok(v)) => Ok(v),
        };

        let elapsed = start.elapsed();
        let (rows, err_code) = match &result {
            Ok(v) => (v.len() as u32, 0u8),
            Err(f) => (0, f.as_error_code()),
        };

        db_log!(
            Info,
            DbPayload {
                db_hash: self.host_hash,
                table_hash: addr_hash,
                query_hash: register_str("read_registers"),
                duration_ns: elapsed.as_nanos() as u64,
                rows_affected: rows,
                op_type: 0, // SELECT / read
                error_code: err_code,
                ..DbPayload::default()
            }
        );

        result
    }

    /// Write a single coil at `address`.
    ///
    /// Emits `db_log!` (op_type=2 UPDATE) with timing.
    pub async fn write_coil(&mut self, address: u16, value: bool) -> Result<(), ModbusFault> {
        let start = std::time::Instant::now();
        let addr_str = format!("coil:{}", address);
        let addr_hash = register_str(&addr_str);

        let raw = self.ctx.write_single_coil(address, value).await;
        let result: Result<(), ModbusFault> = match raw {
            Err(_) => Err(ModbusFault::WriteCoilFailed {
                address,
                exception_code: 0,
            }),
            Ok(Err(ex)) => Err(ModbusFault::WriteCoilFailed {
                address,
                exception_code: ex as u8,
            }),
            Ok(Ok(())) => Ok(()),
        };

        let elapsed = start.elapsed();
        let err_code = match &result {
            Ok(_) => 0u8,
            Err(f) => f.as_error_code(),
        };

        db_log!(
            Info,
            DbPayload {
                db_hash: self.host_hash,
                table_hash: addr_hash,
                query_hash: register_str("write_coil"),
                duration_ns: elapsed.as_nanos() as u64,
                rows_affected: 1,
                op_type: 2, // UPDATE / write
                error_code: err_code,
                ..DbPayload::default()
            }
        );

        result
    }

    /// Write a single holding register at `address`.
    ///
    /// Emits `db_log!` (op_type=2 UPDATE) with timing.
    pub async fn write_register(&mut self, address: u16, value: u16) -> Result<(), ModbusFault> {
        let start = std::time::Instant::now();
        let addr_str = format!("reg:{}", address);
        let addr_hash = register_str(&addr_str);

        let raw = self.ctx.write_single_register(address, value).await;
        let result: Result<(), ModbusFault> = match raw {
            Err(_) => Err(ModbusFault::WriteRegisterFailed {
                address,
                exception_code: 0,
            }),
            Ok(Err(ex)) => Err(ModbusFault::WriteRegisterFailed {
                address,
                exception_code: ex as u8,
            }),
            Ok(Ok(())) => Ok(()),
        };

        let elapsed = start.elapsed();
        let err_code = match &result {
            Ok(_) => 0u8,
            Err(f) => f.as_error_code(),
        };

        db_log!(
            Info,
            DbPayload {
                db_hash: self.host_hash,
                table_hash: addr_hash,
                query_hash: register_str("write_register"),
                duration_ns: elapsed.as_nanos() as u64,
                rows_affected: 1,
                op_type: 2, // UPDATE / write
                error_code: err_code,
                ..DbPayload::default()
            }
        );

        result
    }

    /// Return a reference to the underlying config.
    pub fn config(&self) -> &ModbusConfig {
        &self.config
    }

    /// Return the cached host hash.
    pub fn host_hash(&self) -> u32 {
        self.host_hash
    }
}
