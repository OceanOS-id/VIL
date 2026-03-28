// =============================================================================
// VIL DB Semantic Layer — Compile-Time IR, Runtime Zero-Cost
// =============================================================================
//
// Provider-neutral database surface for application code.
// All abstractions are zero-cost at runtime:
//   - DatasourceRef  → &'static str
//   - TxScope        → 1 byte enum
//   - DbCapability   → u32 bitflag
//   - PortabilityTier → 1 byte enum
//   - CachePolicy    → 8 byte enum
//
// Provider dispatch: 1 vtable call (~1ns) per query.

pub mod cache_policy;
pub mod capability;
pub mod datasource;
pub mod entity;
pub mod error;
pub mod portability;
pub mod provider_trait;
pub mod provisioning;
pub mod repository;
pub mod tx;

pub use cache_policy::CachePolicy;
pub use capability::DbCapability;
pub use datasource::DatasourceRef;
pub use entity::VilEntityMeta;
pub use error::{DbError, DbResult};
pub use portability::PortabilityTier;
pub use provider_trait::{DbProvider, DbQueryExecutor, ProviderExecutor, ToSqlValue};
pub use provisioning::DatasourceRegistry;
pub use tx::TxScope;
