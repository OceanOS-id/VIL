# 704-protocol-modbus-read

Modbus TCP register and coil read.

## What it shows

- `ModbusClient::connect()` to a Modbus TCP server
- `read_registers()` for holding registers (FC03)
- `read_coils()` for digital output status (FC01)
- `db_log!` auto-emitted (op_type=0 SELECT) by `vil_modbus` on every read
- `StdoutDrain::resolved()` output format

## Prerequisites

A Modbus TCP server. Quick start with a simulator:

```bash
docker run -p 502:502 oitc/modbus-server
```

Or use any PLC/gateway that supports Modbus TCP on port 502.

## Run

```bash
cargo run -p example-704-protocol-modbus-read
```

Without a Modbus server, the example prints the config and exits gracefully.
