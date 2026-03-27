// Entity metadata trait — all const, zero runtime cost.

use crate::portability::PortabilityTier;
use crate::cache_policy::CachePolicy;

/// Trait implemented by #[derive(VilEntity)].
/// All associated items are const — zero heap, zero runtime cost.
pub trait VilEntityMeta {
    /// Table name.
    const TABLE: &'static str;
    /// Datasource alias.
    const SOURCE: &'static str;
    /// Primary key field name.
    const PRIMARY_KEY: &'static str;
    /// All field names.
    const FIELDS: &'static [&'static str];
    /// Portability tier.
    const PORTABILITY: PortabilityTier = PortabilityTier::P0;
    /// Default cache policy.
    const CACHE_POLICY: CachePolicy = CachePolicy::None;
}
