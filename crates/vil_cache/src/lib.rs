// =============================================================================
// VIL Cache — Separated from DB Semantic
// =============================================================================
//
// Cache trait + backends:
//   - ShmCacheBackend: ExchangeHeap zero-copy (co-located)
//   - RedisCacheBackend: Redis (distributed)
//
// VilCache is NOT part of VilDb — Redis/cache concerns are separate
// from relational DB concerns.

pub mod cache_trait;
pub mod shm_backend;
pub mod redis_backend;

pub use cache_trait::VilCache;
