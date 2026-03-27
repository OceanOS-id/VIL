// =============================================================================
// VIL Database Plugin — sea-orm (Full ORM)
// =============================================================================
//
// Full ORM integration: Entity, ActiveModel, Relation, Migration.
// Built on top of sea-orm with vil-server plugin system integration.

pub mod pool;
pub mod config;
pub mod metrics;
pub mod migration;

pub use pool::SeaOrmPool;
pub use config::SeaOrmConfig;
