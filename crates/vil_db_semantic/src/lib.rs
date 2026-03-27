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

pub mod datasource;
pub mod tx;
pub mod capability;
pub mod portability;
pub mod cache_policy;
pub mod entity;
pub mod repository;
pub mod provider_trait;
pub mod provisioning;
pub mod error;

pub use datasource::DatasourceRef;
pub use tx::TxScope;
pub use capability::DbCapability;
pub use portability::PortabilityTier;
pub use cache_policy::CachePolicy;
pub use entity::VilEntityMeta;
pub use provider_trait::{DbProvider, DbQueryExecutor, ProviderExecutor, ToSqlValue};
pub use provisioning::DatasourceRegistry;
pub use error::{DbError, DbResult};
