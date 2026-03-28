// =============================================================================
// vil_log::drain — Pluggable drain implementations
// =============================================================================

pub mod fallback;
pub mod file;
pub mod multi;
pub mod null;
pub mod stdout;
pub mod traits;

#[cfg(feature = "clickhouse-drain")]
pub mod clickhouse_drain;

#[cfg(feature = "nats-drain")]
pub mod nats_drain;

pub use fallback::FallbackDrain;
pub use file::{FileDrain, RotationStrategy};
pub use multi::MultiDrain;
pub use null::NullDrain;
pub use stdout::{StdoutDrain, StdoutFormat};
pub use traits::LogDrain;

#[cfg(feature = "clickhouse-drain")]
pub use clickhouse_drain::{ClickHouseConfig, ClickHouseDrain};

#[cfg(feature = "nats-drain")]
pub use nats_drain::{NatsConfig, NatsDrain};
